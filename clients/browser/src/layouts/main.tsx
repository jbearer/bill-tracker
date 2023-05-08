import React from 'react'
import { createUseStyles } from 'react-jss'

import AppMenu from 'components/app-menu'
import NavBar from 'components/nav-bar'

const headerHeight = 64

const useStyles = createUseStyles((theme) => ({
  header: {
    position: 'fixed',
    top: 0,
    height: headerHeight,
    width: '100%',
    backgroundColor: 'white',
    zIndex: 1
  },
  view: {
    marginTop: headerHeight,
    display: 'flex',
    justifyContent: 'center'
  },
  menu: {
    position: 'sticky',
    top: headerHeight,
    alignSelf: 'flex-start'
  },
  appMenu: {
    extend: 'menu',
    flex: '0 1 auto'
  },
  content: {
    flex: '1 1 0',
    overflow: 'auto',
    paddingLeft: '5%',
    paddingRight: '5%'
  },
  viewMenu: {
    extend: 'menu',
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
    <div>
      <header className={classes.header}>
          {/* The navbar is for app-level actions like search and account management. */}
          <NavBar />
      </header>
      <main className={classes.view}>
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
      </main>
    </div>
  )
}
