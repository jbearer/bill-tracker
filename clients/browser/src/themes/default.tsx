import type { Theme, ColorPalette } from './theme'

const colors: ColorPalette = {
  primary: '#7cbf5c',
  primaryLight: '#a3d28c',
  primaryDark: '#5c9b49',

  secondary: '#735726',
  secondaryLight: '#a28860',
  secondaryDark: '#4e3410',

  background: '#ffffff',
  surface: '#ffffff',
  error: '#B00020'
}

const on: ColorPalette = {
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

const defaultTheme: Theme = {
  color: {
    ...colors,
    on
  }
}

export default defaultTheme
