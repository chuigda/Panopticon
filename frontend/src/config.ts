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
  // 直连远程端点，不经过本地代理；需要端点支持浏览器 CORS
  directConnect: boolean

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
  directConnect: z.boolean().default(false),
  chatModel: ModelConfigSchema,
  statusBarModel: ModelConfigSchema,
  memoryModel: ModelConfigSchema,
}).strict()

export const defaultConfig: Config = {
  inlineMessageLimit: 12,
  compressionSize: 8,
  outputLength: 1200,
  directConnect: false,

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

// 代理模式下请求发往同源，由后端按 X-Upstream-Base-Url 转发；直连模式下直接发往模型端点
export function resolveEndpoint(config: Config, model: ModelConfig, pathSuffix = ''): {
  baseURL: string
  headers: Record<string, string>
} {
  if (config.directConnect) {
    return {
      baseURL: model.uri.replace(/\/+$/, '') + pathSuffix,
      headers: { ...model.headers }
    }
  }
  return {
    baseURL: window.location.origin + pathSuffix,
    headers: { 'X-Upstream-Base-Url': model.uri, ...model.headers }
  }
}
