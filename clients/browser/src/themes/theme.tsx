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
  border?: BorderOptions
  activateOnHover?: boolean
}

export interface BorderOptions {
  radius?: string | number
  style?: string
  width?: string | number
  only?: Border[]
}

export enum Border {
  Top,
  Bottom,
  Left,
  Right
}

export type ColorStyle = Record<string, any>

export class Theme {
  _color: ThemeColors

  constructor (color: ThemeColors) {
    this._color = color
  }

  background (opt: ColorOptions = {}): ColorStyle {
    return this.color((p) => p.background, opt)
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
    if (opt.border !== undefined) {
      if (opt.border.only === undefined) {
        style.borderStyle = opt.border.style ?? 'solid'
      } else {
        for (const pos of opt.border.only) {
          switch (pos) {
            case Border.Top:
              style.borderTop = opt.border.style ?? 'solid'
              break
            case Border.Bottom:
              style.borderBottom = opt.border.style ?? 'solid'
              break
            case Border.Left:
              style.borderLeft = opt.border.style ?? 'solid'
              break
            case Border.Right:
              style.borderRight = opt.border.style ?? 'solid'
              break
          }
        }
      }

      style.borderColor = selector(this._color.on)
      style.borderRadius = opt.border.radius ?? 0
      style.borderWidth = opt.border.width ?? '1px'
      style.overflow = 'hidden' // This makes child elements clip to the curved border
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
