import type {
  ContentBlockParam,
  MessageParam,
  TextBlockParam,
  Tool,
  ToolResultBlockParam
} from '@anthropic-ai/sdk/resources'
import { Anthropic } from '@anthropic-ai/sdk'

import { resolveEndpoint, type Config, type ModelConfig } from '../config'
import type { ChatAssistantMessage, ChatMessage } from '../session/message'
import type { ChatTool } from './tool'
import { assert } from '../util'

import simulatorUserPromptCD from '../prompts/simulator.cd.user.xml?raw'

export async function anthropicSimulate(
  config: Config,
  tools: ChatTool[],
  simulatorSystem: string,
  chatMessages: ChatMessage[],
  start: number,
  lastAssistant: number | undefined,
  userString: string,
  signal: AbortSignal
): Promise<void> {
  assert(
    () => lastAssistant === undefined
          || lastAssistant < start
          || chatMessages.slice(lastAssistant + 1).some(m => m.$k === 'user'),
    'There must be a \'user\' message after the last assistant message'
  )

  const client = createClient(config.chatModel)

  const messages: MessageParam[] = []

  let currentMessage: MessageParam | undefined = undefined
  let status: string | undefined = undefined

  if (chatMessages[start].$k !== 'user' && chatMessages[start].$k !== 'memory') {
    currentMessage = {
      role: 'user',
      content: [{
        type: 'text',
        text: `<${userString}><!-- truncated --></${userString}>`
      }]
    }
    messages.push(currentMessage)
  }

  for (let i = start; i < chatMessages.length; i++) {
    const chatMessage = chatMessages[i]
    assert (chatMessage.$k !== 'error', 'Chat message must not be an \'error\'')

    switch (chatMessage.$k) {
      case 'assistant': case 'tool_call': {
        if (!currentMessage || currentMessage.role !== 'assistant') {
          currentMessage = { role: 'assistant', content: [] }
          messages.push(currentMessage)
        }
        break
      }
      case 'user': case 'memory': case 'tool_call_result': {
        if (!currentMessage || currentMessage.role !== 'user') {
          currentMessage = { role: 'user', content: [] }
          messages.push(currentMessage)
        }
        break
      }
    }

    let content = currentMessage!.content as ContentBlockParam[]

    switch (chatMessage.$k) {
      case 'assistant':
        content.push({ type: 'text', text: chatMessage.content })
        break
      case 'user':
        if (status) {
          content.push({
            type: 'text',
            text: `<status>\n${status}\n</status>`
          })
          status = undefined
        }
        content.push({
          type: 'text',
          text: `<${userString}>\n${chatMessage.content}\n</${userString}>`
        })
        if (status) {
          content.push({
            type: 'text',
            text: simulatorUserPromptCD
          })
        }
        break
      case 'memory':
        content.push({
          type: 'text',
          text: `<memory>\n${chatMessage.content}\n</memory>`
        })
        break
      case 'tool_call':
        if (chatMessage.content) {
          content.push({ type: 'text', text: chatMessage.content })
        }
        for (const call of chatMessage.calls) {
          content.push({
            type: 'tool_use',
            name: call.name,
            id: call.id,
            input: call.arguments
          })
        }
        break
      case 'tool_call_result':
        for (const result of chatMessage.results) {
          content.push({
            type: 'tool_result',
            tool_use_id: result.id,
            content: serializeToolContent(result.content),
            is_error: result.isError,
          })
        }
        break
    }

    if (i === lastAssistant) {
      assert(
        chatMessage.$k === 'assistant',
        'The last assistant message must be \'assistant\''
      )
      for (let j = content.length - 1; j >= 0; j--) {
        const contentBlock = content[j]
        if (contentBlock.type === 'text') {
          contentBlock.cache_control = {
            type: 'ephemeral',
            ttl: config.chatModel.cacheTtl
          }
          break
        }
      }

      status = chatMessage.statusBar
    }
  }

  chatMessages.push({
    $k: 'assistant',
    content: '',
    statusBar: '',
    tokenUsage: { inputTokens: 0, cacheInputTokens: 0, cacheCreateTokens: 0, outputTokens: 0 },
    statusBarTokenUsage: { inputTokens: 0, cacheInputTokens: 0, cacheCreateTokens: 0, outputTokens: 0 }
  })
  const generatedMessage = chatMessages[chatMessages.length - 1] as ChatAssistantMessage

  const toolMap = new Map(tools.map(tool => [tool.name, tool]))

  while (true) {
    const stream = client.messages.stream({
      system: [{
        type: 'text',
        text: simulatorSystem,
        cache_control: {
          type: 'ephemeral',
          ttl: config.chatModel.systemCacheTtl ?? config.chatModel.cacheTtl
        }
      }],
      model: config.chatModel.modelName,
      max_tokens: config.chatModel.outputBudget,
      messages,
      tools: tools.length > 0
        ? tools.map(tool => ({
            type: 'custom',
            name: tool.name,
            description: tool.description,
            input_schema: tool.inputJSONSchema as Tool.InputSchema,
          }))
        : undefined,
      thinking: config.chatModel.thinkingEnabled
        ? { type: 'adaptive', display: 'summarized' }
        : { type: 'disabled' },
      output_config: config.chatModel.thinkingEnabled
        ? { effort: config.chatModel.reasoningEffort as any }
        : undefined,

      ...samplingParams(config.chatModel),
      ...config.chatModel.args
    }, { signal })

    for await (const event of stream) {
      switch (event.type) {
        case 'content_block_start':
          if (event.content_block.type === 'text' && generatedMessage.content) {
            generatedMessage.content += '\n\n'
          } else if (event.content_block.type === 'thinking' && generatedMessage.reasoning) {
            generatedMessage.reasoning += '\n\n'
          }
          break
        case 'content_block_delta':
          switch (event.delta.type) {
            case 'text_delta':
              generatedMessage.content += event.delta.text
              break
            case 'thinking_delta':
              generatedMessage.reasoning = (generatedMessage.reasoning ?? '')
                                           + event.delta.thinking
              break
          }
          break
      }
    }

    const finalMessage = await stream.finalMessage()
    messages.push({ role: 'assistant', content: finalMessage.content })

    const tokenUsage = {
      inputTokens: finalMessage.usage.input_tokens,
      cacheInputTokens: finalMessage.usage.cache_read_input_tokens ?? 0,
      cacheCreateTokens: finalMessage.usage.cache_creation_input_tokens ?? 0,
      outputTokens: finalMessage.usage.output_tokens
    }
    generatedMessage.tokenUsage.inputTokens += tokenUsage.inputTokens
    generatedMessage.tokenUsage.cacheInputTokens += tokenUsage.cacheInputTokens
    generatedMessage.tokenUsage.cacheCreateTokens += tokenUsage.cacheCreateTokens
    generatedMessage.tokenUsage.outputTokens += tokenUsage.outputTokens

    if (finalMessage.stop_reason !== 'tool_use') {
      break
    }

    const toolCallContent = generatedMessage.content || undefined
    generatedMessage.content = ''
    const toolUses = finalMessage.content.filter(block => block.type === 'tool_use')
    const executed = await Promise.all(
      toolUses.map(async (toolUse): Promise<{ id: string, content: unknown, isError: boolean }> => {
        const tool = toolMap.get(toolUse.name)
        if (!tool) {
          return { id: toolUse.id, content: `Unknown tool: ${toolUse.name}`, isError: true }
        }
        try {
          const content = await tool.execute(toolUse.input as Record<string, unknown>)
          return { id: toolUse.id, content, isError: false }
        } catch (e) {
          return { id: toolUse.id, content: e instanceof Error ? e.message : String(e), isError: true }
        }
      })
    )

    chatMessages.splice(chatMessages.length - 1, 0, {
      $k: 'tool_call',
      content: toolCallContent,
      calls: toolUses.map(toolUse => ({
        id: toolUse.id,
        name: toolUse.name,
        arguments: toolUse.input as Record<string, unknown>
      })),
      tokenUsage
    }, {
      $k: 'tool_call_result',
      results: executed
    })

    messages.push({
      role: 'user',
      content: executed.map(({ id, content, isError }): ToolResultBlockParam => ({
        type: 'tool_result',
        tool_use_id: id,
        content: serializeToolContent(content),
        is_error: isError,
      }))
    })
  }
}

export async function anthropicStatusBar(
  config: Config,
  statusBarSystem: string,
  chatMessages: ChatMessage[],
  start: number,
  lastAssistant: number | undefined,
  userString: string,
  signal: AbortSignal
): Promise<void> {
  const lastChatMessage = chatMessages[chatMessages.length - 1] as ChatAssistantMessage
  assert(lastChatMessage.$k === 'assistant', 'The last assistant message must be \'assistant\'')

  const client = createClient(config.statusBarModel)

  let userMessage: MessageParam = {
    role: 'user',
    content: []
  }
  let userMessageContent = userMessage.content as TextBlockParam[]

  for (let i = start; i < chatMessages.length; i++) {
    const chatMessage = chatMessages[i]
    assert (chatMessage.$k !== 'error', 'Chat message must not be an \'error\'')

    switch (chatMessage.$k) {
      case 'user':
        userMessageContent.push({
          type: 'text',
          text: `<${userString}>\n${chatMessage.content}\n</${userString}>\n`
        })
        break
      case 'assistant':
        userMessageContent.push({
          type: 'text',
          text: `<simulator>\n${chatMessage.content}\n</simulator>\n`
        })
        if (i === lastAssistant && chatMessage.statusBar) {
          userMessageContent.push({
            type: 'text',
            text: `<status>\n${chatMessage.statusBar}\n</status>\n`
          })
        }
        break
      case 'memory':
        userMessageContent.push({
          type: 'text',
          text: `<memory>\n${chatMessage.content}\n</memory>\n`
        })
        break
    }

    if (lastAssistant !== undefined && i === lastAssistant - 1
        && config.statusBarModel.cacheTtl && userMessageContent.length > 0) {
      userMessageContent[userMessageContent.length - 1].cache_control = {
        type: 'ephemeral',
        ttl: config.statusBarModel.cacheTtl
      }
    }
  }

  const result = await client.messages.create({
    system: [{
      type: 'text',
      text: statusBarSystem,
      cache_control: {
        type: 'ephemeral',
        ttl: config.statusBarModel.systemCacheTtl ?? config.statusBarModel.cacheTtl
      }
    }],
    model: config.statusBarModel.modelName,
    max_tokens: config.statusBarModel.outputBudget,
    messages: [userMessage],
    thinking: config.statusBarModel.thinkingEnabled
      ? { type: 'adaptive', display: 'summarized' }
      : { type: 'disabled' },
    output_config: config.statusBarModel.thinkingEnabled
      ? { effort: config.statusBarModel.reasoningEffort as any }
      : undefined,

    ...samplingParams(config.statusBarModel),
    ...config.statusBarModel.args
  }, { signal })

  lastChatMessage.statusBar = result.content
    .filter(block => block.type === 'text')
    .map(block => block.text.trim())
    .join('\n\n')
  lastChatMessage.statusBarTokenUsage = {
    inputTokens: result.usage.input_tokens,
    cacheInputTokens: result.usage.cache_read_input_tokens ?? 0,
    cacheCreateTokens: result.usage.cache_creation_input_tokens ?? 0,
    outputTokens: result.usage.output_tokens,
  }
}

export async function anthropicMemoryCompress(
  config: Config,
  memorySystem: string,
  chatMessages: ChatMessage[],
  start: number,
  userString: string,
  signal: AbortSignal
): Promise<void> {
  const client = createClient(config.memoryModel)

  let userMessage: MessageParam = {
    role: 'user',
    content: []
  }
  let userMessageContent = userMessage.content as ContentBlockParam[]

  let i = start
  let compressedCount = 0
  for (; i < chatMessages.length; i++) {
    const chatMessage = chatMessages[i]
    assert (chatMessage.$k !== 'error', 'Chat message must not be an \'error\'')

    switch (chatMessage.$k) {
      case 'memory':
        userMessageContent.push({
          type: 'text',
          text: `<memory>\n${chatMessage.content}\n</memory>\n`
        })
        break
      case 'user':
        userMessageContent.push({
          type: 'text',
          text: `<${userString}>\n${chatMessage.content}\n</${userString}>\n`
        })
        break
      case 'assistant':
        if (chatMessage.statusBar.startsWith('TIME:')) {
          const [timestampLine] = chatMessage.statusBar.split('\n', 2)
          userMessageContent.push({
            type: 'text',
            text: `<!-- ${timestampLine} -->\n`
          })
        }
        userMessageContent.push({
          type: 'text',
          text: `<simulator>\n${chatMessage.content}\n</simulator>\n`
        })
        compressedCount += 1
        break
    }

    if (compressedCount >= config.compressionSize) {
      break
    }
  }

  const result = await client.messages.create({
    system: [{
      type: 'text',
      text: memorySystem,
      cache_control: {
        type: 'ephemeral',
        ttl: config.memoryModel.systemCacheTtl ?? config.memoryModel.cacheTtl
      }
    }],
    model: config.memoryModel.modelName,
    max_tokens: config.memoryModel.outputBudget,
    messages: [userMessage],
    thinking: config.memoryModel.thinkingEnabled
      ? { type: 'adaptive', display: 'summarized' }
      : { type: 'disabled' },
    output_config: config.memoryModel.thinkingEnabled
      ? { effort: config.memoryModel.reasoningEffort as any }
      : undefined,

    ...samplingParams(config.memoryModel),
    ...config.memoryModel.args
  }, { signal })

  chatMessages.splice(i + 1, 0, {
    $k: 'memory',
    content: result.content
      .filter(block => block.type === 'text')
      .map(block => block.text.trim())
      .join('\n\n'),
    tokenUsage: {
      inputTokens: result.usage.input_tokens,
      cacheInputTokens: result.usage.cache_read_input_tokens ?? 0,
      cacheCreateTokens: result.usage.cache_creation_input_tokens ?? 0,
      outputTokens: result.usage.output_tokens,
    }
  })
}

function serializeToolContent(content: unknown): string {
  return typeof content === 'string'
    ? content
    : JSON.stringify(content) ?? String(content)
}

function createClient(model: ModelConfig): Anthropic {
  const { baseURL, headers } = resolveEndpoint(model)
  return new Anthropic({
    baseURL,
    apiKey: model.apiKey,
    dangerouslyAllowBrowser: true,
    defaultHeaders: headers
  })
}

function samplingParams(model: ModelConfig) {
  if (model.thinkingEnabled) {
    return {}
  }
  return {
    temperature: model.temperature,
    top_p: model.topP,
    top_k: model.topK
  }
}
