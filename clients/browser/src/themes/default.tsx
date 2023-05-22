import { Theme, type ColorPalette, type OnColorPalette } from './theme'

const colors: ColorPalette = {
  primary: '#7cbf5c',
  primaryLight: '#a3d28c',
  primaryDark: '#5c9b49',

  secondary: '#a056b3',
  secondaryLight: '#d5b6dd',
  secondaryDark: '#8e32a4',

  background: '#ffffff',
  surface: '#ffffff',
  error: '#B00020'
}

const activeDiff = '#0f0f0f'

const active: ColorPalette = {
  primary: subColors(colors.primary, activeDiff),
  primaryLight: subColors(colors.primaryLight, activeDiff),
  primaryDark: addColors(colors.primaryDark, activeDiff),

  secondary: subColors(colors.secondary, activeDiff),
  secondaryLight: subColors(colors.secondaryLight, activeDiff),
  secondaryDark: addColors(colors.secondaryDark, activeDiff),

  background: subColors(colors.background, activeDiff),
  surface: subColors(colors.surface, activeDiff),
  error: subColors(colors.error, activeDiff)
}

const onPalette: ColorPalette = {
  primary: '#000000',
  primaryDark: '#ffffff',
  primaryLight: '#000000',

  secondary: '#ffffff',
  secondaryDark: '#ffffff',
  secondaryLight: '#000000',

  background: '#000000',
  surface: '#000000',
  error: '#ffffff'
}

const on: OnColorPalette = {
  ...onPalette,
  active: onPalette
}

const defaultTheme: Theme = new Theme({
  ...colors,
  active,
  on
})

export default defaultTheme

function addColors (c1: string, c2: string): string {
  const r1 = parseInt(c1.slice(1, 3), 16)
  const g1 = parseInt(c1.slice(3, 5), 16)
  const b1 = parseInt(c1.slice(5, 7), 16)

  const r2 = parseInt(c2.slice(1, 3), 16)
  const g2 = parseInt(c2.slice(3, 5), 16)
  const b2 = parseInt(c2.slice(5, 7), 16)

  const r = toHexByte(Math.min(255, r1 + r2))
  const g = toHexByte(Math.min(255, g1 + g2))
  const b = toHexByte(Math.min(255, b1 + b2))
  return `#${r}${g}${b}`
}

function subColors (c1: string, c2: string): string {
  const r1 = parseInt(c1.slice(1, 3), 16)
  const g1 = parseInt(c1.slice(3, 5), 16)
  const b1 = parseInt(c1.slice(5, 7), 16)

  const r2 = parseInt(c2.slice(1, 3), 16)
  const g2 = parseInt(c2.slice(3, 5), 16)
  const b2 = parseInt(c2.slice(5, 7), 16)

  const r = toHexByte(Math.max(0, r1 - r2))
  const g = toHexByte(Math.max(0, g1 - g2))
  const b = toHexByte(Math.max(0, b1 - b2))
  return `#${r}${g}${b}`
}

function toHexByte (s: number): string {
  return s.toString(16).padStart(2, '0')
}
