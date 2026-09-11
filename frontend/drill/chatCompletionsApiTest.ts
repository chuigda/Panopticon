import OpenAI from 'openai'
import config from './config-drill.json' with { type: 'json' }

const client = new OpenAI({
  baseURL: config.oaiBaseUrl,
  apiKey: config.oaiApiKey
})

const completion = await client.chat.completions.create({
  model: 'gpt-5.6-sol',
  messages: [
    {
      role: 'system',
      content: 'You are a helpful assistant.'
    },
    {
      role: 'user',
      content: 'Use the `interesting_tool` tool and report what you saw.'
    },
    {
      role: 'assistant',
      content: 'Aye, let me use the `interesting_tool` tool and report what I saw.',
      tool_calls: [
        {
          type: 'function',
          id: '114514',
          function: {
            name: 'interesting_tool',
            arguments: JSON.stringify({
              question: 'What is the meaning of life?'
            })
          }
        }
      ]
    },
    {
      role: 'tool',
      tool_call_id: '114514',
      content: 'Striving and self-submission to Allah, the Almighty, the Most Merciful, the Most Compassionate, is the meaning of life.'
    }
  ]
})
console.info(JSON.stringify(completion, null, 2))
