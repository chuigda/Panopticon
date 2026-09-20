import type { Config } from '../config'
import type { SimulatorCHR, AdditionalCHR, PlayerCHR } from '../session/chr'
import { LanguageConfigs } from '../session/chr'

import simulatorSystemTemplateCD from '../prompts/simulator.cd.xml?raw'
import simulatorSystemTemplateMK from '../prompts/simulator.mk.xml?raw'
import statusBarSystemTemplate from '../prompts/status-bar.xml?raw'
import memorySystemTemplate from '../prompts/memory.xml?raw'

export function buildSimulatorSystemPrompt(
  config: Config,
  simulatorCHR: SimulatorCHR,
  additionalCHRs: AdditionalCHR[],
  playerCHR?: PlayerCHR,
): string {
  const base = playerCHR ? simulatorSystemTemplateMK : simulatorSystemTemplateCD

  const additionalTasks: string[] = []
  const additionalCommands: string[] = []
  const worldSettings: string[] = []
  const characterDatabase: string[] = []
  const additionalDatabaseSections: string[] = []
  const additionalBehaviors: string[] = []
  const additionalProhibitions: string[] = []
  const additionalSections: string[] = []

  if (playerCHR) {
    appendStrings(
      additionalDatabaseSections,
      `<pc name="${playerCHR.name}">\n  ${playerCHR.data}\n</pc>`
    )
  }

  if (simulatorCHR.simulator) {
    appendStrings(additionalTasks, simulatorCHR.simulator.tasks)
    appendStrings(additionalCommands, simulatorCHR.simulator.commands)
    appendStrings(worldSettings, simulatorCHR.simulator.world)
    appendStrings(characterDatabase, simulatorCHR.simulator.characters)
    appendStrings(additionalDatabaseSections, simulatorCHR.simulator.database)
    appendStrings(additionalBehaviors, simulatorCHR.simulator.behaviors)
    appendStrings(additionalProhibitions, simulatorCHR.simulator.prohibitions)
    appendStrings(additionalSections, simulatorCHR.simulator.sections)
  }

  for (const additionalCHR of additionalCHRs) {
    if (!additionalCHR.simulator) {
      continue
    }

    appendStrings(additionalTasks, additionalCHR.simulator.tasks)
    appendStrings(additionalCommands, additionalCHR.simulator.commands)
    appendStrings(worldSettings, additionalCHR.simulator.world)
    appendStrings(characterDatabase, additionalCHR.simulator.characters)
    appendStrings(additionalDatabaseSections, additionalCHR.simulator.database)
    appendStrings(additionalBehaviors, additionalCHR.simulator.behaviors)
    appendStrings(additionalProhibitions, additionalCHR.simulator.prohibitions)
    appendStrings(additionalSections, additionalCHR.simulator.sections)
  }

  const languageConfig = LanguageConfigs[simulatorCHR.language]

  const result = base
    .replaceAll('{$UNIVERSE_NAME}', simulatorCHR.universeName)
    .replaceAll('{$LITERAL_WORK_NAME}', simulatorCHR.literalWorkName)
    .replaceAll('{$LANGUAGE_SELECTION}', languageConfig.name)
    .replaceAll('{$LENGTH_INDICATOR}', `${config.outputLength}${languageConfig.wordUnit}`)
    .replaceAll('{$ADDITIONAL_TASKS}', buildIndented(additionalTasks, 4))
    .replaceAll('{$WORLD_SETTINGS}', buildIndented(worldSettings, 6))
    .replaceAll('{$CHARACTER_DATABASE}', buildIndented(characterDatabase, 6))
    .replaceAll('{$ADDITIONAL_DATABASE_SECTIONS}', buildIndented(additionalDatabaseSections, 4))
    .replaceAll('{$ADDITIONAL_BEHAVIORS}', buildIndented(additionalBehaviors, 4))
    .replaceAll('{$ADDITIONAL_PROHIBITIONS}', buildIndented(additionalProhibitions, 4))
    .replaceAll('{$ADDITIONAL_SECTIONS}', buildIndented(additionalSections, 2))

  if (!playerCHR) {
    return result
  }
  else {
    return result
      .replaceAll('{$PC_NAME}', playerCHR.name)
      .replaceAll('{$ADDITIONAL_COMMANDS}', buildIndented(additionalCommands, 6))
  }
}

export function buildStatusBarSystemPrompt(
  simulatorCHR: SimulatorCHR,
  additionalCHRs: AdditionalCHR[],
  playerCHR?: PlayerCHR
): string {
  const worldSettings: string[] = []
  const characterDatabase: string[] = []
  const additionalDatabaseSections: string[] = []
  const statusBarUpdatingRules: string[] = []
  const additionalSections: string[] = []
  const statusBarFormat: string[] = []
  const statusBarExample: string[] = []

  if (playerCHR) {
    appendStrings(
      additionalDatabaseSections,
      `<pc name="${playerCHR.name}">\n  ${playerCHR.data}\n</pc>`
    )
  }

  if (simulatorCHR.simulator) {
    appendStrings(worldSettings, simulatorCHR.simulator.world)
    appendStrings(characterDatabase, simulatorCHR.simulator.characters)
    appendStrings(additionalDatabaseSections, simulatorCHR.simulator.database)
  }
  appendStrings(additionalSections, simulatorCHR.statusBar.sections)
  appendStrings(statusBarUpdatingRules, simulatorCHR.statusBar.rule)
  appendStrings(statusBarFormat, simulatorCHR.statusBar.format)
  appendStrings(statusBarExample, simulatorCHR.statusBar.example)

  for (const additionalCHR of additionalCHRs) {
    if (additionalCHR.simulator) {
      appendStrings(worldSettings, additionalCHR.simulator.world)
      appendStrings(characterDatabase, additionalCHR.simulator.characters)
      appendStrings(additionalDatabaseSections, additionalCHR.simulator.database)
    }

    if (additionalCHR.statusBar) {
      appendStrings(statusBarUpdatingRules, additionalCHR.statusBar.rule)
      appendStrings(statusBarFormat, additionalCHR.statusBar.format)
      appendStrings(statusBarExample, additionalCHR.statusBar.example)
      appendStrings(additionalSections, additionalCHR.statusBar.sections)
    }
  }

  const languageConfig = LanguageConfigs[simulatorCHR.language]

  return statusBarSystemTemplate
    .replaceAll('{$UNIVERSE_NAME}', simulatorCHR.universeName)
    .replaceAll('{$LANGUAGE_SELECTION}', languageConfig.name)
    .replaceAll('{$WORLD_SETTINGS}', buildIndented(worldSettings, 6))
    .replaceAll('{$CHARACTER_DATABASE}', buildIndented(characterDatabase, 6))
    .replaceAll('{$ADDITIONAL_DATABASE_SECTIONS}', buildIndented(additionalDatabaseSections, 4))
    .replaceAll('{$STATUS_BAR_UPDATING_RULES}', buildIndented(statusBarUpdatingRules, 4))
    .replaceAll('{$ADDITIONAL_SECTIONS}', buildIndented(additionalSections, 2))
    .replaceAll('{$STATUS_BAR_FORMAT}', buildIndented(statusBarFormat, 4))
    .replaceAll('{$STATUS_BAR_EXAMPLE}', buildIndented(statusBarExample, 4))
}

export function buildMemorySystemPrompt(
  simulatorCHR: SimulatorCHR,
  additionalCHRs: AdditionalCHR[]
): string {
  const memoryRules: string[] = []
  const additionalSections: string[] = []

  if (simulatorCHR.memory) {
    appendStrings(memoryRules, simulatorCHR.memory.rules)
    appendStrings(additionalSections, simulatorCHR.memory.sections)
  }

  for (const additionalCHR of additionalCHRs) {
    if (additionalCHR.memory) {
      appendStrings(memoryRules, additionalCHR.memory.rules)
      appendStrings(additionalSections, additionalCHR.memory.sections)
    }
  }

  const languageConfig = LanguageConfigs[simulatorCHR.language]

  return memorySystemTemplate
    .replaceAll('{$UNIVERSE_NAME}', simulatorCHR.universeName)
    .replaceAll('{$LANGUAGE_SELECTION}', languageConfig.name)
    .replaceAll('{$MEMORY_RULES}', buildIndented(memoryRules, 4))
    .replaceAll('{$ADDITIONAL_SECTIONS}', buildIndented(additionalSections, 2))
}

function appendStrings(strings: string[], str?: string) {
  if (!str) {
    return
  }

  if (strings.length !== 0 && strings[strings.length - 1] !== '') {
    strings.push('')
  }

  for (const line of str.split('\n')) {
    strings.push(line)
  }
}

function buildIndented(strings: string[], indent: number): string {
  if (strings.length === 0) {
    return ''
  }

  const indentation = ' '.repeat(indent)
  return strings.join('\n' + indentation)
}
