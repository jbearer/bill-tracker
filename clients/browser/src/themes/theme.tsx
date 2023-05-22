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

export type OnColorPalette = {
  active: ColorPalette
} & ColorPalette

export type ThemeColors = {
  on: OnColorPalette
} & OnColorPalette

export interface ColorOptions {
  border?: boolean
  activateOnHover?: boolean
}

export type ColorStyle = Record<string, any>

export class Theme {
  _color: ThemeColors

  constructor (color: ThemeColors) {
    this._color = color
  }

  surface (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.surface, opt)
  }

  primary (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.primary, opt)
  }

  primaryLight (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.primaryLight, opt)
  }

  primaryDark (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.primaryDark, opt)
  }

  secondary (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.secondary, opt)
  }

  secondaryLight (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.secondaryLight, opt)
  }

  secondaryDark (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.secondaryDark, opt)
  }

  color (selector: (palette: ColorPalette) => string, opt: ColorOptions = {}): ColorStyle {
    const style: ColorStyle = {
      backgroundColor: selector(this._color),
      color: selector(this._color.on)
    }
    if (opt.border ?? false) {
      style.borderColor = selector(this._color.on)
    }
    if (opt.activateOnHover ?? false) {
      style['&:hover'] = {
        backgroundColor: selector(this._color.active),
        color: selector(this._color.on.active)
      }
    }
    return style
  }
}
