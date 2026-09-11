import { z } from 'zod'
import { load } from 'js-toml'

export type SimulatorKind = 'cd' | 'mk'
export type AddonKind = SimulatorKind | '*'
export type LanguageSelector = 'zh_CN' | 'en_US'

export interface LanguageConfig {
  name: string
  wordUnit: string
}

export const LanguageConfigs: Record<LanguageSelector, LanguageConfig> = {
  zh_CN: { name: '简体中文', wordUnit: '汉字' },
  en_US: { name: 'English', wordUnit: 'words' },
}

export interface SimulatorCHR {
  kind: SimulatorKind

  universeName: string
  literalWorkName: string
  prologue?: string
  language: LanguageSelector

  statusBar: StatusBarConfig
  simulator?: SimulatorConfig
  memory?: MemorySummarizerConfig
}

export interface PlayerCHR {
  name: string,
  data: string
}

export interface AdditionalCHR {
  kind: AddonKind

  id: string
  name?: string
  statusBar?: StatusBarConfig
  simulator?: SimulatorConfig
  memory?: MemorySummarizerConfig
}

export interface StatusBarConfig {
  format: string
  example?: string
  rule?: string
  sections?: string
}

export interface SimulatorConfig {
  tasks?: string
  commands?: string
  world?: string
  characters?: string
  database?: string
  behaviors?: string
  prohibitions?: string
  sections?: string
}

export interface MemorySummarizerConfig {
  rules?: string
  sections?: string
}

export const LanguageSelectorSchema = z.enum(['zh_CN', 'en_US'])

export const StatusBarConfigSchema = z.object({
  format: z.string(),
  example: z.string().optional(),
  rule: z.string().optional(),
  sections: z.string().optional(),
})

export const SimulatorConfigSchema = z.object({
  tasks: z.string().optional(),
  commands: z.string().optional(),
  world: z.string().optional(),
  characters: z.string().optional(),
  database: z.string().optional(),
  behaviors: z.string().optional(),
  prohibitions: z.string().optional(),
  sections: z.string().optional(),
})

export const MemorySummarizerConfigSchema = z.object({
  rules: z.string().optional(),
  sections: z.string().optional(),
})

export const SimulatorCHRSchema = z.object({
  kind: z.enum(['cd', 'mk']),
  universeName: z.string(),
  literalWorkName: z.string(),
  prologue: z.string().optional(),
  language: LanguageSelectorSchema,
  statusBar: StatusBarConfigSchema,
  simulator: SimulatorConfigSchema.optional(),
  memory: MemorySummarizerConfigSchema.optional(),
})

export const PlayerCHRSchema = z.object({
  name: z.string(),
  data: z.string(),
})

export const AdditionalCHRSchema = z.object({
  kind: z.enum(['cd', 'mk', '*']),
  id: z.string(),
  name: z.string().optional(),
  statusBar: StatusBarConfigSchema.optional(),
  simulator: SimulatorConfigSchema.optional(),
  memory: MemorySummarizerConfigSchema.optional(),
})

export function parseSimulatorCHR(tomlString: string): SimulatorCHR {
  return SimulatorCHRSchema.parse(load(tomlString))
}

export function parsePlayerCHR(tomlString: string): PlayerCHR {
  return PlayerCHRSchema.parse(load(tomlString))
}


export function parseAdditionalCHR(tomlString: string): AdditionalCHR {
  return AdditionalCHRSchema.parse(load(tomlString))
}
