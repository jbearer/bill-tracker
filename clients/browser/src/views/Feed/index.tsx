import React from 'react'

import MainLayout from 'layouts/main'

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
    <MainLayout>
      <div>
        {type} content
      </div>
    </MainLayout>
  )
}
