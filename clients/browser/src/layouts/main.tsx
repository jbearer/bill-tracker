import React from 'react'
import { createUseStyles } from 'react-jss'

import AppMenu from 'components/app-menu'

const useStyles = createUseStyles((theme) => ({
  view: {
    display: 'flex',
    justifyContent: 'center'
  },
  appMenu: {
    flex: '0 1 auto'
  },
  content: {
    flex: '1 1 0',
    overflow: 'auto'
  },
  viewMenu: {
    flex: '0 1 256px'
  }
}))

interface MainLayoutProps {
  menu?: React.ReactNode
  children: React.ReactNode
}

export default function MainLayout ({ children, menu }: MainLayoutProps): JSX.Element {
  const classes = useStyles()
  return (
    <div className={classes.view}>
      <div className={classes.appMenu}>
        {/* The app menu is for app-level navigation. It renders as a sidebar on the left. */}
        <AppMenu />
      </div>
      <div className={classes.content}>
        {/* Content for the current view goes here. */}
        {children}
      </div>
      <div id="view-menu" className={classes.viewMenu}>
        {/* View-level navigation goes in a sidebar on the right.

            By default it is empty and serves as margin to center the content panel. But some
            views will populate it (via a portal). For example, the search results view populates
            this container with tabs to view results for bills, people, etc.
        */}
        {menu}
      </div>
    </div>
  )
}
