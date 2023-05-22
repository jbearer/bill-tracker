import React, { useEffect, useRef } from 'react'

export type Inside = (node: Node | null) => void

interface Props {
  onClickAway: () => void
  render: (inside: Inside) => React.ReactNode
}

export function ClickAwayListener (props: Props): JSX.Element {
  const ref = useRef<Node | null>(null)
  useEffect(() => {
    function handleClickOutside (event: Event): void {
      if (ref.current !== null) {
        if (!ref.current.contains(event.target as Node)) {
          props.onClickAway()
        }
      }
    }
    document.addEventListener('mouseup', handleClickOutside)
    return () => {
      document.removeEventListener('mouseup', handleClickOutside)
    }
  })
  return <>{
    props.render((node) => {
      ref.current = node
    })
  }</>
}
