import React from 'react'
import { Outlet, ScrollRestoration } from 'react-router-dom'
import { createUseStyles } from 'react-jss'

import NavBar from 'components/nav-bar'

const useStyles = createUseStyles((theme) => ({
  main: {
    display: 'flex',
    flexDirection: 'column'
  },
  header: {
    flex: 'auto'
  },
  view: {
    position: 'absolute',
    top: '78px',
    left: 0,
    right: 0,
    bottom: 0
  }
}))

export default function App (): JSX.Element {
  const classes = useStyles()
  return (
    <main className={classes.main}>
      <header className={classes.header}>
        {/* The navbar is for app-level actions like search and account management. */}
        <NavBar />
      </header>
      <div className={classes.view}>
        <Outlet />
      </div>
      <ScrollRestoration />
    </main>
  )
}
