import type {
  ChatCompletionAssistantMessageParam,
  ChatCompletionMessageParam,
  ChatCompletionMessageToolCall,
  ChatCompletionTool,
} from 'openai/resources/chat/completions'
import type { ChatTokenUsage, ChatAssistantMessage, ChatMessage } from '../session/message'
import { resolveEndpoint, type Config, type ModelConfig } from '../config'
import type { ChatTool } from './tool'
import OpenAI from 'openai'
import { assert } from '../util'

export async function openaiSimulate(
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

  const client = createClient(config, config.chatModel)
  const messages = buildSimulationMessages(
    simulatorSystem,
    chatMessages,
    start,
    lastAssistant,
    userString
  )

  chatMessages.push({
    $k: 'assistant',
    content: '',
    statusBar: '',
    tokenUsage: emptyTokenUsage(),
    statusBarTokenUsage: emptyTokenUsage()
  })
  const generatedMessage = chatMessages[chatMessages.length - 1] as ChatAssistantMessage
  const toolMap = new Map(tools.map(tool => [tool.name, tool]))

  while (true) {
    const stream = await client.chat.completions.create({
      model: config.chatModel.modelName,
      messages,
      max_completion_tokens: config.chatModel.outputBudget,
      stream: true,
      stream_options: { include_usage: true },
      tools: tools.length > 0 ? createToolDefinitions(tools) : undefined,
      ...samplingParams(config.chatModel),
      ...config.chatModel.args
    }, { signal })

    const streamedToolCalls: Array<OpenAIToolCall | undefined> = []
    let finishReason: string | null = null
    let usage: OpenAIUsage | null = null

    for await (const chunk of stream) {
      usage = chunk.usage ?? usage
      const choice = chunk.choices[0]
      if (!choice) continue

      finishReason = choice.finish_reason ?? finishReason
      const delta = choice.delta as typeof choice.delta & { reasoning_content?: string }
      if (delta.content) {
        generatedMessage.content += delta.content
      }
      if (delta.reasoning_content) {
        generatedMessage.reasoning = (generatedMessage.reasoning ?? '') + delta.reasoning_content
      }

      for (const toolCallDelta of delta.tool_calls ?? []) {
        const toolCall = streamedToolCalls[toolCallDelta.index] ?? {
          id: '',
          name: '',
          arguments: ''
        }
        streamedToolCalls[toolCallDelta.index] = toolCall
        if (toolCallDelta.id) toolCall.id = toolCallDelta.id
        if (toolCallDelta.function?.name) toolCall.name += toolCallDelta.function.name
        if (toolCallDelta.function?.arguments) toolCall.arguments += toolCallDelta.function.arguments
      }
    }

    const toolCalls = streamedToolCalls.filter(
      (toolCall): toolCall is OpenAIToolCall => Boolean(toolCall)
    )
    const tokenUsage = tokenUsageFromUsage(usage)
    addTokenUsage(generatedMessage.tokenUsage, tokenUsage)

    const assistantMessage: ChatCompletionAssistantMessageParam = {
      role: 'assistant',
      content: generatedMessage.content || null,
      ...(toolCalls.length > 0
        ? { tool_calls: toolCalls.map(toChatCompletionToolCall) }
        : {})
    }
    if (generatedMessage.reasoning && toolCalls.length > 0) {
      ;(assistantMessage as ChatCompletionAssistantMessageParam & { reasoning_content?: string }).reasoning_content =
        generatedMessage.reasoning
    }
    messages.push(assistantMessage)

    // Some providers report finish_reason 'stop' even when tool_calls are present.
    if (finishReason !== 'tool_calls' && toolCalls.length === 0) {
      break
    }
    assert(toolCalls.length > 0, 'The model requested tool calls without providing any tool calls')

    const parsedToolCalls = toolCalls.map(toolCall => ({
      ...toolCall,
      parsedArguments: parseToolArguments(toolCall.arguments)
    }))
    const toolCallContent = generatedMessage.content || undefined
    generatedMessage.content = ''
    const executed = await Promise.all(
      parsedToolCalls.map(async (toolCall): Promise<{ id: string, content: unknown, isError: boolean }> => {
        const tool = toolMap.get(toolCall.name)
        if (!tool) {
          return { id: toolCall.id, content: `Unknown tool: ${toolCall.name}`, isError: true }
        }
        try {
          const content = await tool.execute(toolCall.parsedArguments)
          return { id: toolCall.id, content, isError: false }
        } catch (e) {
          return { id: toolCall.id, content: e instanceof Error ? e.message : String(e), isError: true }
        }
      })
    )

    chatMessages.splice(chatMessages.length - 1, 0, {
      $k: 'tool_call',
      content: toolCallContent,
      calls: parsedToolCalls.map(toolCall => ({
        id: toolCall.id,
        name: toolCall.name,
        arguments: toolCall.parsedArguments
      })),
      tokenUsage
    }, {
      $k: 'tool_call_result',
      results: executed
    })

    messages.push(...executed.map(({ id, content, isError }) => ({
      role: 'tool' as const,
      tool_call_id: id,
      content: serializeToolContent(content, isError)
    })))
  }
}

export async function openaiStatusBar(
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

  const client = createClient(config, config.statusBarModel)
  const userContent: string[] = []

  for (let i = start; i < chatMessages.length; i++) {
    const chatMessage = chatMessages[i]
    assert(chatMessage.$k !== 'error', 'Chat message must not be an \'error\'')

    switch (chatMessage.$k) {
      case 'user':
        userContent.push(`<${userString}>\n${chatMessage.content}\n</${userString}>\n`)
        break
      case 'assistant':
        userContent.push(`<simulator>\n${chatMessage.content}\n</simulator>\n`)
        if (i === lastAssistant && chatMessage.statusBar) {
          userContent.push(`<status>\n${chatMessage.statusBar}\n</status>\n`)
        }
        break
      case 'memory':
        userContent.push(`<memory>\n${chatMessage.content}\n</memory>\n`)
        break
    }
  }

  const result = await client.chat.completions.create({
    model: config.statusBarModel.modelName,
    messages: [
      { role: 'system', content: statusBarSystem },
      { role: 'user', content: userContent.join('') }
    ],
    max_completion_tokens: config.statusBarModel.outputBudget,
    ...samplingParams(config.statusBarModel),
    ...config.statusBarModel.args
  }, { signal })

  lastChatMessage.statusBar = result.choices[0]?.message.content?.trim() ?? ''
  lastChatMessage.statusBarTokenUsage = tokenUsageFromUsage(result.usage)
}

export async function openaiMemoryCompress(
  config: Config,
  memorySystem: string,
  chatMessages: ChatMessage[],
  start: number,
  userString: string,
  signal: AbortSignal
): Promise<void> {
  const client = createClient(config, config.memoryModel)
  const userContent: string[] = []
  let i = start
  let compressedCount = 0

  for (; i < chatMessages.length; i++) {
    const chatMessage = chatMessages[i]
    assert(chatMessage.$k !== 'error', 'Chat message must not be an \'error\'')

    switch (chatMessage.$k) {
      case 'memory':
        userContent.push(`<memory>\n${chatMessage.content}\n</memory>\n`)
        break
      case 'user':
        userContent.push(`<${userString}>\n${chatMessage.content}\n</${userString}>\n`)
        break
      case 'assistant':
        if (chatMessage.statusBar.startsWith('TIME:')) {
          const [timestampLine] = chatMessage.statusBar.split('\n', 2)
          userContent.push(`<!-- ${timestampLine} -->\n`)
        }
        userContent.push(`<simulator>\n${chatMessage.content}\n</simulator>\n`)
        compressedCount += 1
        break
    }

    if (compressedCount >= config.compressionSize) {
      break
    }
  }

  const result = await client.chat.completions.create({
    model: config.memoryModel.modelName,
    messages: [
      { role: 'system', content: memorySystem },
      { role: 'user', content: userContent.join('') }
    ],
    max_completion_tokens: config.memoryModel.outputBudget,
    ...samplingParams(config.memoryModel),
    ...config.memoryModel.args
  }, { signal })

  chatMessages.splice(i + 1, 0, {
    $k: 'memory',
    content: result.choices[0]?.message.content?.trim() ?? '',
    tokenUsage: tokenUsageFromUsage(result.usage)
  })
}

function createClient(config: Config, model: ModelConfig): OpenAI {
  const { baseURL, headers } = resolveEndpoint(config, model, '/v1')
  return new OpenAI({
    baseURL,
    apiKey: model.apiKey,
    dangerouslyAllowBrowser: true,
    defaultHeaders: headers
  })
}

function buildSimulationMessages(
  simulatorSystem: string,
  chatMessages: ChatMessage[],
  start: number,
  lastAssistant: number | undefined,
  userString: string
): ChatCompletionMessageParam[] {
  const messages: ChatCompletionMessageParam[] = [
    { role: 'system', content: simulatorSystem }
  ]
  let status: string | undefined

  if (chatMessages[start].$k !== 'user' && chatMessages[start].$k !== 'memory') {
    messages.push({
      role: 'user',
      content: [{
        type: 'text',
        text: `<${userString}><!-- truncated --></${userString}>`
      }]
    })
  }

  for (let i = start; i < chatMessages.length; i++) {
    const chatMessage = chatMessages[i]
    assert(chatMessage.$k !== 'error', 'Chat message must not be an \'error\'')

    switch (chatMessage.$k) {
      case 'assistant':
        messages.push({ role: 'assistant', content: chatMessage.content })
        if (i === lastAssistant) {
          status = chatMessage.statusBar
        }
        break
      case 'user': {
        const content = status
          ? `<status>\n${status}\n</status>\n<${userString}>\n${chatMessage.content}\n</${userString}>`
          : `<${userString}>\n${chatMessage.content}\n</${userString}>`
        messages.push({ role: 'user', content })
        status = undefined
        break
      }
      case 'memory':
        messages.push({ role: 'user', content: `<memory>\n${chatMessage.content}\n</memory>` })
        break
      case 'tool_call':
        messages.push({
          role: 'assistant',
          content: chatMessage.content ?? null,
          tool_calls: chatMessage.calls.map(call => ({
            type: 'function',
            id: call.id,
            function: {
              name: call.name,
              arguments: JSON.stringify(call.arguments)
            }
          }))
        })
        break
      case 'tool_call_result':
        for (const result of chatMessage.results) {
          messages.push({
            role: 'tool',
            tool_call_id: result.id,
            content: serializeToolContent(result.content, result.isError)
          })
        }
        break
    }
  }

  return messages
}

function createToolDefinitions(tools: ChatTool[]): ChatCompletionTool[] {
  return tools.map(tool => ({
    type: 'function',
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.inputJSONSchema
    }
  }))
}

function toChatCompletionToolCall(toolCall: OpenAIToolCall): ChatCompletionMessageToolCall {
  assert(toolCall.id && toolCall.name, 'The model returned an incomplete tool call')
  return {
    type: 'function',
    id: toolCall.id,
    function: {
      name: toolCall.name,
      arguments: toolCall.arguments
    }
  }
}

function parseToolArguments(argumentsText: string): Record<string, unknown> {
  const parsed: unknown = JSON.parse(argumentsText)
  assert(
    typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed),
    'Tool arguments must be a JSON object'
  )
  return parsed as Record<string, unknown>
}

function serializeToolContent(content: unknown, isError = false): string {
  const serialized = typeof content === 'string'
    ? content
    : JSON.stringify(content) ?? String(content)
  return isError ? `[Tool error]\n${serialized}` : serialized
}

function samplingParams(model: ModelConfig): Record<string, unknown> {
  return {
    reasoning_effort: model.thinkingEnabled ? model.reasoningEffort : undefined,
    temperature: model.temperature,
    top_p: model.topP,
    frequency_penalty: model.frequencyPenalty,
    presence_penalty: model.presencePenalty
  }
}

type OpenAIUsage = {
  prompt_tokens: number
  completion_tokens: number
  prompt_tokens_details?: { cached_tokens?: number | null } | null
}

type OpenAIToolCall = {
  id: string
  name: string
  arguments: string
}

function tokenUsageFromUsage(usage: OpenAIUsage | null | undefined): ChatTokenUsage {
  return {
    inputTokens: usage?.prompt_tokens ?? 0,
    cacheInputTokens: usage?.prompt_tokens_details?.cached_tokens ?? 0,
    cacheCreateTokens: 0,
    outputTokens: usage?.completion_tokens ?? 0
  }
}

function emptyTokenUsage(): ChatTokenUsage {
  return {
    inputTokens: 0,
    cacheInputTokens: 0,
    cacheCreateTokens: 0,
    outputTokens: 0
  }
}

function addTokenUsage(target: ChatTokenUsage, source: ChatTokenUsage): void {
  target.inputTokens += source.inputTokens
  target.cacheInputTokens += source.cacheInputTokens
  target.cacheCreateTokens += source.cacheCreateTokens
  target.outputTokens += source.outputTokens
}
