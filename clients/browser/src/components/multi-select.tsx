import React, { useState, useRef, useEffect, useMemo } from 'react'
import { createUseStyles } from 'react-jss'
import Fuse from 'fuse.js'

import { ClickAwayListener } from 'components/click-away-listener'
import { type Theme, Border } from 'themes/theme'

const useStyles = createUseStyles((theme: Theme) => ({
  container: {
    width: '100%',
    cursor: 'pointer',
    ...theme.surface()
  },
  selectionList: {
    display: 'flex',
    flexWrap: 'wrap'
  },
  selectionItem: {
    padding: '2px',
    margin: '2px',
    ...theme.secondaryLight({ border: { radius: '5px', width: 0 } })
  },
  selectionName: {
    padding: '0px 2px'
  },
  deselectButton: {
    padding: '0px 2px',
    ...theme.secondary()
  },
  placeholder: {
    padding: '2px',
    margin: '2px',
    color: 'gray'
  },
  modal: {
    position: 'absolute',
    boxShadow: '0.02em 0.02em',
    ...theme.surface({ border: { radius: '5px' } })
  },
  filter: {
    ...theme.surface()
  },
  filterInput: {
    ...theme.surface({ border: { radius: '5px', width: 0 } }),
    padding: '5px',
    margin: '5px'
  },
  options: {
    maxHeight: '50vh',
    overflow: 'auto'
  },
  option: {
    cursor: 'pointer',
    padding: '5px',
    ...theme.surface({ border: { only: [Border.Top] }, activateOnHover: true })
  },
  selectedOption: {
    extend: 'option',
    ...theme.primaryLight({ activateOnHover: true })
  }
}))

export type Filter<T> = (query: string) => T[]

export interface MultiSelectProps<T> {
  filter: Filter<T>
  placeholder?: T
  onChange?: (selected: T[]) => void
}

export function MultiSelect<T extends React.ReactNode> (props: MultiSelectProps<T>): JSX.Element {
  const classes = useStyles()
  const toggles = useRef<Toggles<T>>(new Toggles())
  const setDropdownActive = useRef<(active: boolean) => void>()

  const showDropdown = (active: boolean): void => {
    if (setDropdownActive.current !== undefined) {
      setDropdownActive.current(active)
    }
  }

  return <ClickAwayListener
    onClickAway={() => { showDropdown(false) }}
    render={(inside) =>
      <div ref={inside} className={classes.container}>
        <SelectedItems
          toggles={toggles.current}
          onClick={() => { showDropdown(true) }}
          onChange={props.onChange}
          placeholder={props.placeholder}
        />
        <Dropdown
          toggles={toggles.current}
          filter={props.filter}
          setActive={(f) => { setDropdownActive.current = f }}
        />
      </div>
    }
  />
}

interface SelectedItemsProps<T> {
  toggles: Toggles<T>
  onClick: () => void
  onChange?: (selected: T[]) => void
  placeholder?: T
}

function SelectedItems<T extends React.ReactNode> (props: SelectedItemsProps<T>): JSX.Element {
  const classes = useStyles()
  const selected = props.toggles.useSelected((selected) => {
    if (props.onChange !== undefined) {
      props.onChange(selected.map((toggle) => toggle.node()))
    }
  })

  const list = []
  if (selected.length === 0) {
    list.push(<span key='placeholder' className={classes.placeholder}>{props.placeholder ?? 'None'}</span>)
  } else {
    selected.forEach((item, i) => {
      list.push(
        <React.Fragment key={i}>
          <SelectionItem>
            {item}
          </SelectionItem>
        </React.Fragment>
      )
    })
  }

  return <div className={classes.selectionList} onClick={props.onClick}>
    {list}
  </div>
}

interface SelectionItemProps<T> {
  children: Toggle<T>
}

function SelectionItem<T extends React.ReactNode> ({ children }: SelectionItemProps<T>): JSX.Element {
  const classes = useStyles()
  return <div className={classes.selectionItem}>
    <span className={classes.selectionName}>{children.node()}</span>
    <span className={classes.deselectButton} onClick={(e) => {
      e.stopPropagation()
      children.toggle(false)
    }}>x</span>
  </div>
}

interface DropdownProps<T> {
  toggles: Toggles<T>
  setActive: (f: (active: boolean) => void) => void
  filter: Filter<T>
}

function Dropdown<T extends React.ReactNode> (props: DropdownProps<T>): JSX.Element {
  const [active, setActive] = useState(false)

  props.setActive(setActive)

  if (!active) {
    return <> </>
  } else {
    return <DropdownModal {...props} />
  }
}

function DropdownModal<T extends React.ReactNode> (props: DropdownProps<T>): JSX.Element {
  const [query, setQuery] = useState('')
  const matches = useMemo(() => props.filter(query), [props.filter, query])
  const toggles = props.toggles.useToggles(matches)
  const classes = useStyles()

  const items = toggles.map((toggle) =>
    <ItemToggle key={toggle.key()}>
      {toggle}
    </ItemToggle>
  )

  return <div className={classes.modal}>
    <div className={classes.filter}>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          if (toggles.length !== 0) {
            toggles[0].toggle()
          }
        }}
      >
        <input className={classes.filterInput} autoFocus type="text" value={query}
          onChange={(event) => { setQuery(event.target.value) }}
        />
      </form>
    </div>
    <div className={classes.options}>
      {items}
    </div>
  </div>
}

interface ItemToggleProps<T> {
  children: Toggle<T>
}

function ItemToggle<T extends React.ReactNode> (props: ItemToggleProps<T>): JSX.Element {
  const classes = useStyles()
  const selected = props.children.useSelected()

  const className = selected ? classes.selectedOption : classes.option
  return (
    <div className={className}
      onClick={() => { props.children.toggle() }}
    >
      {props.children.node()}
    </div>
  )
}

export function fuzzyFilter (options: string[]): (query: string) => string[] {
  const fuse = new Fuse(options)
  return (query) => query === '' ? options : fuse.search(query).map((res) => res.item)
}

class Toggle<T> {
  _key: number
  _node: T
  _selected: boolean
  _onToggleListeners: Array<(selected: boolean) => void>

  constructor (key: number, value: T) {
    this._key = key
    this._node = value
    this._selected = false
    this._onToggleListeners = []
  }

  key (): number {
    return this._key
  }

  node (): T {
    return this._node
  }

  selected (): boolean {
    return this._selected
  }

  toggle (selected: boolean | undefined = undefined): void {
    this._selected = selected ?? !this._selected
    this._onToggleListeners.forEach((f) => { f(this._selected) })
  }

  useSelected (): boolean {
    const initialValue = this._selected
    const [selected, setSelected] = useState(initialValue)
    useEffect(() => {
      if (this._selected !== initialValue) {
        // In the unlikely event that the selected state has changed between creating the state
        // variable and this callback firing, fire an event to correct it.
        setSelected(this._selected)
      }

      // Listen for future changes to the state.
      this._onToggleListeners.push(setSelected)
      return () => {
        // Remove the listener when this component is closed.
        const index = this._onToggleListeners.indexOf(setSelected)
        this._onToggleListeners.splice(index, 1)
      }
    }, [setSelected])
    return selected
  }
}

class Toggles<T> {
  _toggles: Map<T, Toggle<T>>
  _onToggleListeners: Array<(toggle: Toggle<T>) => void>
  _newToggles: Array<Toggle<T>>

  constructor () {
    this._toggles = new Map()
    this._onToggleListeners = []
    this._newToggles = []
  }

  _get (item: T): Toggle<T> {
    let toggle = this._toggles.get(item)
    if (toggle === undefined) {
      const newToggle = new Toggle(this._toggles.size, item)
      this._toggles.set(item, newToggle)
      this._newToggles.push(newToggle)
      toggle = newToggle
    }
    return toggle
  }

  _registerNewToggles (): void {
    for (const toggle of this._newToggles) {
      // Trigger the listeners if this item's selected state ever changes.
      toggle._onToggleListeners.push((_) => {
        this._onToggleListeners.forEach((f) => { f(toggle) })
      })
    }
    this._newToggles = []
  }

  useToggles (items: T[]): Array<Toggle<T>> {
    const toggles = items.map((item) => this._get(item))
    useEffect(() => { this._registerNewToggles() })
    return toggles
  }

  useSelected (onChange: ((selected: Array<Toggle<T>>) => void) | undefined = undefined): Array<Toggle<T>> {
    const [selected, setSelected] = useState(() => {
      const selected = []
      for (const toggle of this._toggles.values()) {
        if (toggle.selected()) {
          selected.push(toggle)
        }
      }
      return selected
    })
    useEffect(() => {
      this._onToggleListeners.push((toggle) => {
        const newSelected = toggle.selected()
          ? selected.concat([toggle])
          : selected.filter((sel) => sel !== toggle)
        setSelected(newSelected)
        if (onChange !== undefined) {
          onChange(newSelected)
        }
      })
    })
    return selected
  }
}
