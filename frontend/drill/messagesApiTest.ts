import Anthropic from '@anthropic-ai/sdk'
import config from './config-drill.json' with { type: 'json' }

const client = new Anthropic({
  baseURL: config.baseUrl,
  apiKey: config.apiKey
})

const message = await client.messages.create({
  max_tokens: 1024,
  model: 'claude-fable-5-1',
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
      content: [
        {
          type: 'text',
          text: '<memory>The `interesting_tool` tool actually has a single parameter `question`. Since it is called interesting tool, it must be very funny to call it twice with the same questions.</memory>'
        },
        {
          type: 'text',
          text: 'Aye, let me use the `interesting_tool` tool and report what I saw.'
        },
        {
          type: 'tool_use',
          id: '114514',
          name: 'interesting_tool',
          input: {
            question: 'What is the meaning of life?'
          }
        },
        {
          type: 'tool_use',
          id: '114515',
          name: 'interesting_tool',
          input: {
            question: 'What is the meaning of life?'
          }
        }
      ]
    },
    {
      role: 'user',
      content: [
        {
          type: 'tool_result',
          tool_use_id: '114514',
          content: 'Striving and self-submission to Allah, the Almighty, the Most Merciful, the Most Compassionate, is the meaning of life.'
        },
        {
          type: 'tool_result',
          tool_use_id: '114515',
          content: 'You have already asked me this question once, right? (2:70) They said, "Call upon your Lord to make clear to us what it is. Indeed, [all] cows look alike to us. And indeed we, if Allāh wills, will be guided."'
        }
      ]
    }
  ]
})
console.info(message)
