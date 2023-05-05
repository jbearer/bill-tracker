import React from 'react'
import { Outlet, ScrollRestoration } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

import NavBar from './components/nav-bar'
import SideMenu from './components/side-menu'

const useStyles = createUseStyles((theme) => ({
  main: {},
  header: {
    backgroundColor: 'white',
    height: '50px'
  },
  app: {
    position: 'absolute',
    top: '50px',
    left: 0,
    right: 0,
    bottom: 0,
    display: 'flex',
    justifyContent: 'center'
  },
  sidebar: {
    flex: '0 1 auto'
  },
  content: {
    flex: '1 1 0',
    overflow: 'auto'
  }
}))

export default function App (): JSX.Element {
  const classes = useStyles()
  return (
    <main className={classes.main}>
      <header className={classes.header}>
        <NavBar />
      </header>
      <div className={classes.app}>
        <div className={classes.sidebar}>
          <SideMenu />
        </div>
        <div className={classes.content}>
          <Outlet />
        </div>
        <ScrollRestoration />
      </div>
    </main>
  )
}
