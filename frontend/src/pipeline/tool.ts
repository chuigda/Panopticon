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

const questionSchema = z.object({
  prompt: z.string(),
  options: stringArray.optional(),
})

const askQuestionSchema = z.object({
  questions: z.array(questionSchema).min(1),
})

// 兼容旧的单问题格式 { prompt, options }
const askQuestionInput = z.preprocess((v) => {
  if (v && typeof v === 'object' && !('questions' in v) && 'prompt' in v) return { questions: [v] }
  return v
}, askQuestionSchema)

export function createAskQuestionTool(askQuestion: AskQuestionCallback): ChatTool {
  return {
    name: 'ask_question',
    description: [
      'Ask the user one or more clarifying questions before generating story content.',
      'Use when you are unsure about intent, creative direction, or need to confirm an important decision.',
      'Each question needs a clear, concise prompt. If there are obvious choices, list them in options. The user may pick one, or type a free-form answer.',
      'Returns an array of { prompt, answer } in the same order as the questions.',
    ].join('\n'),
    inputSchema: askQuestionInput,
    inputJSONSchema: z.toJSONSchema(askQuestionSchema, { target: 'draft-07' }),
    execute: async (args) => {
      const { questions } = askQuestionInput.parse(args)
      const answers = await Promise.all(questions.map(({ prompt, options }) => askQuestion({ prompt, options })))
      return questions.map(({ prompt }, i) => ({ prompt, answer: answers[i] }))
    },
  }
}
