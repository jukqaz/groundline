# Platform Support

GroundLine supports:

- macOS on Apple Silicon
- Linux

macOS validation uses a local fake home. Linux validation uses Docker with
`linux/arm64` as the default container platform on Apple Silicon hosts.

Required baseline:

- Python 3
- git
- POSIX-style paths
- rg when available
- curl only for explicit network radar checks

GroundLine scripts should avoid real user home writes during tests.
