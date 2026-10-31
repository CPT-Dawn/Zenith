# No ambient animation by default

GTK keeps a frame-clock tick running for as long as any infinite CSS animation is active, so a single "breathing" glow makes the bar redraw every frame forever. The default motion level is therefore `subtle`: transitions happen only in response to an event (hover, track change, threshold crossed). Continuous effects such as v1's flowing RGB border are opt-in through `motion = "ambient"`, and the docs state their CPU/GPU cost.
