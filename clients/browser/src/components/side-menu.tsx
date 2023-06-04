import React from 'react'
import { Link, NavLink } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

import { type Theme, Border } from 'themes/theme'

const useStyles = createUseStyles((theme: Theme) => ({
  sidebar: {
    display: 'flex',
    flexDirection: 'column',
    margin: '12px',
    ...theme.background(),

    '& > :not(:first-child)': {
      // Section dividers.
      ...theme.background({ border: { only: [Border.Top] } }),
      marginTop: '12px',
      paddingTop: '12px'
    }
  },
  item: {
    flex: '30px',

    paddingLeft: '12px',
    paddingRight: '12px',
    display: 'flex',
    alignItems: 'center',

    ...theme.background({ border: { radius: '5px', width: 0 } })
  },
  section: {
    display: 'flex',
    flexDirection: 'column',

    '& > a': {
      extend: 'item',
      textDecoration: 'none',
      ...theme.background({ activateOnHover: true })
    },

    '& > a.active': {
      ...theme.primary({ activateOnHover: true })
    }
  },
  header: {
    paddingLeft: '12px',
    paddingRight: '12px'
  },
  footer: {
    display: 'flex',
    flexDirection: 'column',
    paddingLeft: '12px',
    paddingRight: '12px',

    '& > a': {
      color: '#828282',
      textDecoration: 'none'
    }
  }
}))

interface SideMenuSectionProps {
  children: React.ReactNode
}

export function SideMenuSection ({ children }: SideMenuSectionProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.section}>{children}</div>
}

interface SideMenuFooterProps {
  children: React.ReactNode
}

export function SideMenuFooter ({ children }: SideMenuFooterProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.footer}>{children}</div>
}

interface SideMenuHeaderProps {
  children: React.ReactNode
}

export function SideMenuHeader ({ children }: SideMenuHeaderProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.header}><b>{children}</b></div>
}

interface SideMenuLinkProps {
  to: string
  children: React.ReactNode
}

export function SideMenuLink ({ to, children }: SideMenuLinkProps): JSX.Element {
  return <Link to={to}>{children}</Link>
}

interface SideMenuNavLinkProps {
  to: string
  children: React.ReactNode
}

export function SideMenuNavLink ({ to, children }: SideMenuNavLinkProps): JSX.Element {
  return <NavLink to={to}>{children}</NavLink>
}

interface SideMenuItemProps {
  children: React.ReactNode
}

export function SideMenuItem ({ children }: SideMenuItemProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.item}>{children}</div>
}

interface SideMenuProps {
  children: React.ReactNode
}

export function SideMenu ({ children }: SideMenuProps): JSX.Element {
  const classes = useStyles()
  return <div className={classes.sidebar}>{children}</div>
}
