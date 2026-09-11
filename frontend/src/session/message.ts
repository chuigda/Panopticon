export interface ChatMessageBase<K extends string> { $k: K }

export interface ChatUserMessage extends ChatMessageBase<'user'> { content: string }

export interface ChatTokenUsage {
  inputTokens: number
  cacheInputTokens: number
  cacheCreateTokens: number
  outputTokens: number
}

export interface ChatAssistantMessage extends ChatMessageBase<'assistant'> {
  reasoning?: string
  content: string
  statusBar: string

  tokenUsage: ChatTokenUsage
  statusBarTokenUsage: ChatTokenUsage
}

export interface ChatToolCall {
  id: string
  name: string
  arguments: Record<string, unknown>
}

export interface ChatToolCallResult {
  id: string
  content: unknown
  isError: boolean
}

export interface ChatToolCallMessage extends ChatMessageBase<'tool_call'> {
  content?: string
  calls: ChatToolCall[]
  tokenUsage: ChatTokenUsage
}

export interface ChatToolCallResultMessage extends ChatMessageBase<'tool_call_result'> {
  results: ChatToolCallResult[]
}

export interface ChatMemoryMessage extends ChatMessageBase<'memory'> {
  content: string
  tokenUsage: ChatTokenUsage
}

export interface ChatErrorMessage extends ChatMessageBase<'error'> {
  error: string
}

export type ChatMessage =
    ChatUserMessage
  | ChatAssistantMessage
  | ChatToolCallMessage
  | ChatToolCallResultMessage
  | ChatMemoryMessage
  | ChatErrorMessage
