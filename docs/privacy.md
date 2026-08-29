# Privacy

GroundLine processes explicit local inputs on the user's machine. The public
plugin does not transmit data, configure a destination, create a persistent
device identity, or run automatically in the background.

The local audit surface reads Codex's state database in read-only mode and
returns aggregate counts. It does not emit prompt text, response text, task
titles, repository names, filesystem paths, configuration values, credentials,
or database rows. `project-audit` counts recognized Codex configuration surfaces
without reading their contents.

Users control every input path and invocation. Deleting GroundLine removes no
Codex data because the public plugin does not own a data store.
