import { z } from 'zod'

export interface ChatTool {
  name: string
  description: string
  inputSchema: z.ZodType
  inputJSONSchema: Record<string, unknown>
  execute: (args: Record<string, unknown>) => Promise<unknown>
}

export interface AskQuestionCallback {
  (args: { prompt: string; options?: string[] }): Promise<string>
}

// 模型偶尔会把 options 序列化成 JSON 字符串，这里容错
const stringArray = z.preprocess((v) => {
  if (typeof v === 'string') {
    try { return JSON.parse(v) } catch { return v }
  }
  return v
}, z.array(z.string()))

const askQuestionSchema = z.object({
  prompt: z.string(),
  options: stringArray.optional(),
})

export function createAskQuestionTool(askQuestion: AskQuestionCallback): ChatTool {
  return {
    name: 'ask_question',
    description: [
      'Ask the a clarifying question before generating story content.',
      'Use when you are unsure about intent, creative direction, or need to confirm an important decision.',
      'Provide a clear, concise prompt. If there are obvious choices, list them in options — the user may pick one or type a free-form answer.',
    ].join('\n'),
    inputSchema: askQuestionSchema,
    inputJSONSchema: z.toJSONSchema(askQuestionSchema, { target: 'draft-07' }),
    execute: async (args) => {
      const { prompt, options } = askQuestionSchema.parse(args)
      return askQuestion({ prompt, options })
    },
  }
}
