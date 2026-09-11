import { z } from 'zod'

export interface ModelConfig {
  uri: string
  apiKey: string

  protocol: 'chat-completions' | 'messages'
  modelName: string
  outputBudget: number
  thinkingEnabled: boolean

  reasoningEffort?: string
  temperature?: number
  topP?: number
  topK?: number
  frequencyPenalty?: number
  presencePenalty?: number
  cacheTtl?: '5m' | '1h'
  systemCacheTtl?: '5m' | '1h'

  headers?: Record<string, string>
  args?: Record<string, any>
}

export interface Config {
  inlineMessageLimit: number
  compressionSize: number
  outputLength: number

  chatModel: ModelConfig
  statusBarModel: ModelConfig
  memoryModel: ModelConfig
}

export const ModelConfigSchema = z.object({
  uri: z.string(),
  apiKey: z.string(),
  protocol: z.enum(['chat-completions', 'messages']),
  modelName: z.string(),
  outputBudget: z.number(),
  thinkingEnabled: z.boolean(),
  reasoningEffort: z.string().optional(),
  temperature: z.number().optional(),
  topP: z.number().optional(),
  topK: z.number().optional(),
  frequencyPenalty: z.number().optional(),
  presencePenalty: z.number().optional(),
  cacheTtl: z.enum(['5m', '1h']).optional(),
  systemCacheTtl: z.enum(['5m', '1h']).optional(),
  headers: z.record(z.string(), z.string()).optional(),
  args: z.record(z.string(), z.unknown()).optional(),
}).strict()

export const ConfigSchema = z.object({
  inlineMessageLimit: z.number(),
  compressionSize: z.number(),
  outputLength: z.number(),
  chatModel: ModelConfigSchema,
  statusBarModel: ModelConfigSchema,
  memoryModel: ModelConfigSchema,
}).strict()

export const defaultConfig: Config = {
  inlineMessageLimit: 16,
  compressionSize: 12,
  outputLength: 1512,

  chatModel: {
    uri: '',
    apiKey: '',
    protocol: 'messages',
    modelName: 'claude-opus-4-6',
    outputBudget: 8192,
    thinkingEnabled: true,
    reasoningEffort: 'medium',
    cacheTtl: '5m',
    systemCacheTtl: '1h',
  },
  statusBarModel: {
    uri: 'https://api.deepseek.com',
    apiKey: '',
    protocol: 'chat-completions',
    modelName: 'deepseek-v4-flash',
    outputBudget: 8192,
    thinkingEnabled: true,
    reasoningEffort: 'medium',
    temperature: 0.05
  },
  memoryModel: {
    uri: 'https://api.deepseek.com',
    apiKey: '',
    protocol: 'chat-completions',
    modelName: 'deepseek-v4-flash',
    outputBudget: 8192,
    thinkingEnabled: true,
    reasoningEffort: 'medium',
    temperature: 0.05
  }
}
