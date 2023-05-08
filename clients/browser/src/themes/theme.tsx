export interface ColorPalette {
  primary: string
  primaryLight: string
  primaryDark: string
  secondary: string
  secondaryLight: string
  secondaryDark: string
  background: string
  surface: string
  error: string
}

export type ThemeColors = {
  on: ColorPalette
} & ColorPalette

export interface Theme {
  color: ThemeColors
}
