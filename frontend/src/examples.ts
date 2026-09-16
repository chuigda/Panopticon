export type ExampleTree = { [name: string]: ExampleTree | string }

export async function fetchExampleTree(): Promise<ExampleTree> {
  const res = await fetch('/examples')
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return (await res.json()) as ExampleTree
}

export async function downloadExample(name: string, url: string) {
  const res = await fetch(url)
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  const a = document.createElement('a')
  a.href = URL.createObjectURL(await res.blob())
  a.download = name
  a.click()
  URL.revokeObjectURL(a.href)
}
