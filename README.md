# Lavalink and Andesite interoperable wrapper

### Mention this project was initially forked from [lavalink-rs](https://gitlab.com/nitsuga5124/lavalink-rs/), [license](https://gitlab.com/nitsuga5124/lavalink-rs/-/blob/master/LICENSE)

## Why to use it?

### We provide few differences and improvements

- Multiple node support, all nodes are managed by a Cluster
- Automatically node reconnection with configurable reconnect tries, if a node fails to reconnect, it is removed from cluster automatically
- Fully event driven track scheduling. Instead of spawning a new task per each track, we preferred to listen to lavalink events when scheduling tracks, this means with a single task all tracks are scheduled, thus reducing a lot of workload with lots of queued tracks
- Node balancing based on number of players. When a new player is created with `get_best()`, the returned node will always be the one with few players
- Node shared data, provided from cluster at Node's initialization

## Adding to a project
To add `andelink` to a project, just add the following to your *Cargo.toml*
```toml
[dependencies.andelink]
git = "https://github.com/AlvaroMS25/AndeLink.git"
branch = "master"
```

## Usage example
### To see an usage example, take a look on `examples` folder