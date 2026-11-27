import type { Provider } from './types'
import { steam } from './steam'
import { epic } from './epic'
import { gog } from './gog'
import { battlenet } from './battlenet'
import { ea } from './ea'
import { ubisoft } from './ubisoft'
import { xbox } from './xbox'

export const PROVIDERS: Provider[] = [steam, epic, gog, battlenet, ea, ubisoft, xbox]
export type { Provider, ProviderContext, Detection } from './types'
