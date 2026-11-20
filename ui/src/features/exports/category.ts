import type { Protection } from './api'

export const categoryLabel: Record<Protection, string> = {
  provider_managed: 'Provider-managed',
  independent_backup: 'Independent backup',
  user_output: 'User output',
  unknown_owner: 'Unknown ownership',
  shared: 'Shared references',
  in_use: 'In use',
  incomplete: 'Partial coverage',
}
