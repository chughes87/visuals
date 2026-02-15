# Fractal Explorer

An interactive Mandelbrot set renderer in Clojure using Quil.

## Setup

Make sure you have [Leiningen](https://leiningen.org/) installed, then:

```bash
cd fractal-explorer
lein run
```

## Controls

- **Click** - Zoom in on a point (2x zoom, centers on click)
- **Space** - Cycle through color schemes (classic, fire, ocean, psychedelic)
- **+/-** - Increase/decrease iteration depth (more detail vs speed)
- **R** - Reset view

## Customization Ideas

Open `src/fractal_explorer/core.clj` and try:

### Add new color schemes
Add to the `color-schemes` map:
```clojure
:your-scheme (fn [iter max-iter]
               (if (= iter max-iter)
                 [0 0 0]
                 (let [t (/ iter max-iter)]
                   [r g b])))  ; your color logic
```

### Change the fractal
Replace `mandelbrot-iterations` with other escape-time fractals:
- Julia sets (pass different c constants)
- Burning Ship
- Tricorn

### Adjust rendering
- Change `(range 0 w 2)` to `(range 0 w 1)` for higher quality (slower)
- Modify initial zoom/center in `setup`
- Add zoom-out with right-click

### Try smooth coloring
Replace the color scheme functions with:
```clojure
(let [smooth-iter (+ iter 1 (- (/ (Math/log (Math/log (+ x2 y2)))
                                   (Math/log 2))))]
  ...)
```

## REPL Development

```bash
lein repl
```

Then in the REPL:
```clojure
(require '[fractal-explorer.core :as f] :reload)
(f/-main)
```

Modify code and reload to see changes!
