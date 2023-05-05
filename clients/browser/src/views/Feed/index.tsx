import React from 'react'

interface FeedProps {
  type?: FeedType
}

export enum FeedType {
  Home,
  Recent,
  Trending,
  History,
}

export default function Feed ({ type }: FeedProps): JSX.Element {
  type ??= FeedType.Home
  return (
    <div>
      {type} content
    </div>
  )
}
