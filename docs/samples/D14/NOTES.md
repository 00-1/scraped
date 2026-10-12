# D14 samples: notes

A wayfarer on seed 42 walks south from the start for most of a day, turning when barred, then comes back with `go back` (`scraped-lang samples D14 --bot wayback`). Once in clear weather, once in fog.

- **Clear:** heading south stops each time something comes into view ("You stop: the damaged waystation has come into view"), or on reaching it ("You reach the plain ruins"). The way back retraces every leg, and every stop is told as known ("Here again: scrub."). Landmarks change bearing as the walker passes them (the holy city north, middle, then far, then horizon).
- **Fog:** every leg out says the walker is unsure of the way ("With nothing to steer by…"). On the way back, over ground already walked, it says nothing of the kind, and the walk ends at the start.
- Barriers are seen before they stop the walk ("A river lies ahead to the south.").
