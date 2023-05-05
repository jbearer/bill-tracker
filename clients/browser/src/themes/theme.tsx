interface ThemeColors {
  primary: string
  primaryLight: string
  primaryDark: string
  secondary: string
  secondaryLight: string
  secondaryDark: string
}

type ColorPalette = {
  on: ThemeColors
} & ThemeColors

export interface Theme {
  color: ColorPalette
}
