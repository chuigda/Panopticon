import type { Config } from '../config'
import type { SimulatorCHR, AdditionalCHR, PlayerCHR } from '../session/chr'
import type { ChatAssistantMessage, ChatMessage } from '../session/message'
import { assert } from '../util'
import { anthropicMemoryCompress, anthropicSimulate, anthropicStatusBar } from './api_anthropic'
import { openaiMemoryCompress, openaiSimulate, openaiStatusBar } from './api_openai'
import { buildMemorySystemPrompt, buildSimulatorSystemPrompt, buildStatusBarSystemPrompt } from './prompt'
import type { ChatTool } from './tool'

export type PipelineStage = 'simulate' | 'statusBar' | 'memory'
export type StageCallback = (stage: PipelineStage) => void

export function removeErrorMessages(chatMessages: ChatMessage[]) {
  for (let i = chatMessages.length - 1; i >= 0; i--) {
    if (chatMessages[i].$k === 'error') {
      chatMessages.splice(i, 1)
    }
  }
}

export async function pipelineGenerate(
  config: Config,
  tools: ChatTool[],
  simulatorCHR: SimulatorCHR,
  additionalCHRs: AdditionalCHR[],
  playerCHR: PlayerCHR | undefined,
  chatMessages: ChatMessage[],
  discardToolCalls: boolean,
  onStage: StageCallback,
  signal: AbortSignal
): Promise<void> {
  removeErrorMessages(chatMessages)

  const userString = playerCHR ? 'player' : 'conducting-department'
  const simulatorSystem = buildSimulatorSystemPrompt(config, simulatorCHR, additionalCHRs, playerCHR)

  while (chatMessages.length > 0
         && (chatMessages[chatMessages.length - 1].$k === 'assistant'
             || (discardToolCalls
                 && (chatMessages[chatMessages.length - 1].$k === 'tool_call'
                     || chatMessages[chatMessages.length - 1].$k === 'tool_call_result')))) {
    chatMessages.pop()
  }

  let start: number = 0
  let inlineMessageCount: number = 0
  let lastAssistant: number | undefined = undefined
  for (let i = chatMessages.length - 1; i >= 0; i--) {
    const chatMessage = chatMessages[i]

    if (chatMessage.$k === 'assistant') {
      if (lastAssistant === undefined) {
        lastAssistant = i
      }
      inlineMessageCount += 1
    }

    if (chatMessage.$k === 'memory') {
      start = i
      break
    }
  }

  const simulateFn = config.chatModel.protocol === 'messages'
    ? anthropicSimulate
    : openaiSimulate

  onStage('simulate')
  try {
    await simulateFn(config, tools, simulatorSystem, chatMessages, start, lastAssistant, userString, signal)
  } catch (e) {
    chatMessages.push({
      $k: 'error',
      error: `Simulation generation error:\n${e}`
    })
    return
  }

  const statusBarFn = config.statusBarModel.protocol === 'messages'
    ? anthropicStatusBar
    : openaiStatusBar
  const statusSystem = buildStatusBarSystemPrompt(simulatorCHR, additionalCHRs, playerCHR)

  onStage('statusBar')
  try {
    await statusBarFn(config, statusSystem, chatMessages, start, lastAssistant, userString, signal)
  } catch (e) {
    if (lastAssistant !== undefined) {
      const lastAssistantMessage = chatMessages[lastAssistant] as ChatAssistantMessage
      const thisAssistantMessage = chatMessages[chatMessages.length - 1] as ChatAssistantMessage
      thisAssistantMessage.statusBar = lastAssistantMessage.statusBar
    }

    chatMessages.push({
      $k: 'error',
      error: `Status bar generation error:\n${e}`
    })
    return
  }

  if (inlineMessageCount + 1 > config.inlineMessageLimit) {
    const memoryCompressFn = config.memoryModel.protocol === 'messages'
      ? anthropicMemoryCompress
      : openaiMemoryCompress
    const memorySystem = buildMemorySystemPrompt(simulatorCHR, additionalCHRs)

    onStage('memory')
    try {
      await memoryCompressFn(config, memorySystem, chatMessages, start, userString, signal)
    } catch (e) {
      chatMessages.push({
        $k: 'error',
        error: `Memory compression error:\n${e}`
      })
      return
    }
  }
}

export async function regenerateStatusBar(
  config: Config,
  simulatorCHR: SimulatorCHR,
  additionalCHRs: AdditionalCHR[],
  playerCHR: PlayerCHR | undefined,
  chatMessages: ChatMessage[],
  onStage: StageCallback,
  signal: AbortSignal
): Promise<void> {
  removeErrorMessages(chatMessages)

  assert(
    chatMessages[chatMessages.length - 1].$k === 'assistant',
    'The last chat message must be an assistant message'
  )

  let start: number = 0
  let lastAssistant: number | undefined = undefined
  for (let i = chatMessages.length - 2; i >= 0; i--) {
    const chatMessage = chatMessages[i]

    if (chatMessage.$k === 'assistant') {
      if (lastAssistant === undefined) {
        lastAssistant = i
      }
    }

    if (chatMessage.$k === 'memory') {
      start = i
      break
    }
  }

  const userString = playerCHR ? 'player' : 'conducting-department'
  const statusBarFn = config.statusBarModel.protocol === 'messages'
    ? anthropicStatusBar
    : openaiStatusBar
  const statusSystem = buildStatusBarSystemPrompt(simulatorCHR, additionalCHRs, playerCHR)

  onStage('statusBar')
  try {
    await statusBarFn(config, statusSystem, chatMessages, start, lastAssistant, userString, signal)
  } catch (e) {
    chatMessages.push({
      $k: 'error',
      error: `Status bar regeneration error:\n${e}`
    })
    return
  }
}

export async function manualCompressMemory(
  config: Config,
  simulatorCHR: SimulatorCHR,
  additionalCHRs: AdditionalCHR[],
  playerCHR: PlayerCHR | undefined,
  chatMessages: ChatMessage[],
  onStage: StageCallback,
  signal: AbortSignal
): Promise<void> {
  removeErrorMessages(chatMessages)

  let start: number = 0
  for (let i = chatMessages.length - 1; i >= 0; i--) {
    const chatMessage = chatMessages[i]

    if (chatMessage.$k === 'memory') {
      start = i
      break
    }
  }

  const userString = playerCHR ? 'player' : 'conducting-department'
  const memoryCompressFn = config.memoryModel.protocol === 'messages'
    ? anthropicMemoryCompress
    : openaiMemoryCompress
  const memorySystem = buildMemorySystemPrompt(simulatorCHR, additionalCHRs)

  onStage('memory')
  try {
    await memoryCompressFn(config, memorySystem, chatMessages, start, userString, signal)
  } catch (e) {
    chatMessages.push({
      $k: 'error',
      error: `Manual memory compression error:\n${e}`
    })
    return
  }
}
