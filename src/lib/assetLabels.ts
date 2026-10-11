// Asset metadata comes from the native backend; UI labels stay in English.
export function modelDescription(id: string) {
  return id.includes('large') || id === 'turbo'
    ? 'Higher accuracy for multilingual audio.'
    : 'Compact model for faster transcription.'
}

export function runtimeName(id: string) {
  return id === 'cpu' ? 'CPU' : 'NVIDIA CUDA'
}

export function runtimeDescription(id: string) {
  return id === 'cpu' ? 'Works on any computer.' : 'GPU acceleration for NVIDIA graphics.'
}
