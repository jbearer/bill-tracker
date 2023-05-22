import React from 'react'
import { Link } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

import { type Theme } from 'themes/theme'

const useStyles = createUseStyles((theme: Theme) => ({
  sidebar: {
    display: 'flex',
    flexDirection: 'column',
    margin: '12px',

    '& > :not(:first-child)': {
      // Section dividers.
      borderTop: '1px solid',
      marginTop: '12px',
      paddingTop: '12px'
    }
  },
  item: {
    flex: '30px',

    paddingLeft: '12px',
    paddingRight: '12px',
    borderRadius: '5px',

    display: 'flex',
    alignItems: 'center',

    ...theme.surface()
  },
  section: {
    display: 'flex',
    flexDirection: 'column',

    '& > a': {
      extend: 'item',
      textDecoration: 'none',
      '&:hover': {
        backgroundColor: '#f2f2f2'
      }
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
