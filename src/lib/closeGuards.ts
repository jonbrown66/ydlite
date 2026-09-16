type CloseGuard = () => Promise<boolean>

const guards = new Set<CloseGuard>()

export function registerCloseGuard(guard: CloseGuard) {
  guards.add(guard)
  return () => { guards.delete(guard) }
}

export async function canCloseWindow() {
  for (const guard of guards) {
    if (!await guard()) return false
  }
  return true
}
