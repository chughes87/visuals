# Fractal Explorer

A modular visual programming environment for creating fractal art in Clojure. Think of it like a modular synthesizer, but for visuals — you route signals through processing modules to create different visual "patches".

## Setup

Make sure you have [Leiningen](https://leiningen.org/) installed, then:

```bash
lein run
```

## Controls

- **1-5** - Load preset patches
- **Space** - Cycle through all presets
- **Click** - Zoom in on a point
- **+/-** - Increase/decrease iteration depth
- **R** - Reset to default
- **Q** - Quit

## Preset Patches

1. **Classic Mandelbrot** - Pure Mandelbrot with classic coloring
2. **Psychedelic Julia** - Animated Julia set with hue shifting
3. **Trippy Mandelbrot** - Ripples, echoes, and particles
4. **Burning Ship Trails** - Burning Ship fractal with motion blur
5. **Noise Field** - Animated Perlin noise

## Architecture

The system is built around 4 core module types:

### Generators (`src/generators.clj`)

Generate raw visual signals:

- `MandelbrotGenerator` - Classic Mandelbrot set
- `JuliaGenerator` - Julia set fractals (with configurable c parameter)
- `BurningShipGenerator` - Burning Ship fractal
- `NoiseGenerator` - Perlin noise fields

### Effects (`src/effects.clj`)

Process and transform visual signals:

- `ColorMapper` - Map iteration values to colors (classic, fire, ocean, psychedelic)
- `MotionBlur` - Create trailing/ghosting effects
- `RippleDistortion` - Sinusoidal wave distortion
- `Echo` - Multi-layer offset copies
- `ParticleSystem` - Spawn particles at high-value points
- `HueShift` - Rotate colors
- `BrightnessContrast` - Adjust brightness/contrast
- `Feedback` - Blend with previous frames

### Modulators (`src/modulators.clj`)

Control parameters over time. There are two kinds:

**Scalar modulators** — return a single value in the range `[-1.0, 1.0]` based on the current params (specifically `:time`). They are building blocks; use them as inputs to `ModMatrix`.

- `LFO` - Low frequency oscillators (sine, triangle, square, saw)
- `Envelope` - ADSR-style envelope triggered by `:trigger-time` in params
- `MouseModulator` - Mouse position mapped to `[-1, 1]` along `:x` or `:y`
- `AudioModulator` - Placeholder for audio reactivity (currently uses Perlin noise)
- `RandomWalk` - Smooth random parameter drift

**`ModMatrix`** — the only modulator that directly modifies the params map. It takes a list of mappings, each binding a scalar modulator to a specific parameter key with an output range:

```clojure
{:modulator <Modulator>   ; any scalar modulator
 :param     :zoom         ; the params key to control
 :min       1.0           ; maps modulator output -1.0 → this value
 :max       5.0}          ; maps modulator output +1.0 → this value
```

At each frame, `ModMatrix` calls `modulate` on each child modulator (getting a scalar), scales it into `[min, max]`, and `assoc`s it onto the params map. Only `ModMatrix` instances are effective when placed in a patch's top-level modulators list; all other modulator types must be nested inside one.

### Patches (`src/patch.clj`)

Combine generators, effects, and modulators into signal chains:

```
Generator → Effect 1 → Effect 2 → ... → Renderer
              ↑          ↑
         Modulator 1  Modulator 2
```

## Signal Flow Examples

```
CLASSIC:
Mandelbrot → ColorMapper(classic) → Screen

TRIPPY:
Mandelbrot → ColorMapper(ocean) → Ripple → Echo → Particles → Screen
                                     ↑
                                 LFO(sine)

PSYCHEDELIC:
Julia → ColorMapper(psychedelic) → HueShift → Screen
                                      ↑
                                  LFO(sine)

INTERACTIVE:
BurningShip → ColorMapper(fire) → MotionBlur → Screen
                                      ↑
                                  MouseMod(x)
```

## REPL Development

```bash
lein repl
```

### Create a custom patch

```clojure
(require '[generators :as gen])
(require '[effects :as fx])
(require '[modulators :as mod])
(require '[patch :as patch])

(def my-patch
  (patch/create-patch
    ;; Generator
    (gen/make-mandelbrot)

    ;; Effect chain
    [(fx/make-color-mapper :fire)
     (fx/make-ripple 0.1 20 1.5)
     (fx/make-particles 0.9 0.02)]

    ;; Modulators
    [(mod/make-lfo 0.5 :sine)   ; Modulates over time
     (mod/make-mouse-mod :x)]   ; Mouse X controls a parameter

    ;; Initial parameters
    {:width 800
     :height 600
     :center-x -0.5
     :center-y 0.0
     :zoom 1.0
     :max-iter 100
     :time 0}))

;; Process the patch
(def result (patch/process-patch my-patch))

;; Modify the patch
(def modified-patch
  (-> my-patch
      (patch/add-effect (fx/make-echo 3 10 10 2.0))
      (patch/add-modulator (mod/make-lfo 0.3 :triangle))))
```

### Use the modulation matrix

Route multiple modulators to multiple parameters:

```clojure
(def mod-matrix-patch
  (patch/create-patch
    (gen/make-mandelbrot)
    [(fx/make-color-mapper :psychedelic)]

    [(mod/make-mod-matrix
      [{:modulator (mod/make-lfo 0.2 :sine)
        :param :zoom
        :min 1.0
        :max 5.0}

       {:modulator (mod/make-lfo 0.3 :triangle)
        :param :center-x
        :min -1.0
        :max 0.0}

       {:modulator (mod/make-mouse-mod :x)
        :param :max-iter
        :min 50
        :max 200}])]

    {...}))
```

### Modify and reload

Edit any source file, then in the REPL:

```clojure
(require '[core :as f] :reload)
```

## Extending the System

### Create a new generator

Implement the `Generator` protocol:

```clojure
(defrecord MyFractalGenerator [param1 param2]
  gen/Generator
  (generate [this params]
    ;; Return vector of {:x :y :value :max-value} maps
    (let [{:keys [width height]} params]
      (for [y (range 0 height 2)
            x (range 0 width 2)]
        {:x x
         :y y
         :value (your-fractal-function x y param1 param2)
         :max-value 100}))))
```

### Create a new effect

Implement the `Effect` protocol:

```clojure
(defrecord MyEffect [intensity]
  fx/Effect
  (process [this pixel-data params]
    (map (fn [pixel]
           (update pixel :value * intensity))
         pixel-data)))
```

### Try smooth coloring

Replace the color scheme functions with:

```clojure
(let [smooth-iter (+ iter 1 (- (/ (Math/log (Math/log (+ x2 y2)))
                                   (Math/log 2))))]
  ...)
```

## Philosophy

This architecture lets you:

- **Experiment** - Try different combinations in the REPL
- **Compose** - Stack modules like Lego blocks
- **Extend** - Add new generators/effects/modulators
- **Share** - Save patches as data structures
- **Perform** - Live-code visual performances
