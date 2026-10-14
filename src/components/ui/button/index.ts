import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Button } from './Button.vue'

export const buttonVariants = cva(
  'ui-button inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-ui-control text-sm font-medium disabled:pointer-events-none disabled:opacity-45',
  {
    variants: {
      variant: {
        default: 'ui-button-primary',
        secondary: 'ui-button-secondary',
        outline: 'ui-button-outline',
        ghost: 'ui-button-ghost',
        danger: 'ui-button-danger',
      },
      size: {
        default: 'h-9 px-3',
        sm: 'h-8 px-3',
        icon: 'size-9 p-0',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
)

export type ButtonVariants = VariantProps<typeof buttonVariants>
