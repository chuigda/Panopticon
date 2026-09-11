export interface Cell<T> {
  value: T
}

export function cell<T>(value: T): Cell<T> {
  return { value }
}

export function assert(condition: unknown, message?: string): asserts condition {
  if (typeof condition === 'function') {
    condition = condition()
  }

  if (!condition) {
    throw new Error(message ?? 'Assertion failed')
  }
}
