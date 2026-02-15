# Fractal Explorer - Modular Visual Synthesizer

A modular visual programming environment for creating fractal art in Clojure. Think of it like a modular synthesizer, but for visuals - you route signals through processing modules to create different visual "patches".

## Architecture

The system is built around 4 core module types:

### 🎛️ **Generators** (`modules/generators.clj`)

Generate raw visual signals:

- `MandelbrotGenerator` - Classic Mandelbrot set
- `JuliaGenerator` - Julia set fractals (with configurable c parameter)
- `BurningShipGenerator` - Burning Ship fractal
- `NoiseGenerator` - Perlin noise fields

### 🎨 **Effects** (`modules/effects.clj`)

Process and transform visual signals:

- `ColorMapper` - Map iteration values to colors (classic, fire, ocean, psychedelic)
- `MotionBlur` - Create trailing/ghosting effects
- `RippleDistortion` - Sinusoidal wave distortion
- `Echo` - Multi-layer offset copies
- `ParticleSystem` - Spawn particles at high-value points
- `HueShift` - Rotate colors
- `BrightnessContrast` - Adjust brightness/contrast
- `Feedback` - Blend with previous frames

### 🎚️ **Modulators** (`modules/modulators.clj`)

Control parameters over time:

- `LFO` - Low frequency oscillators (sine, triangle, square, saw)
- `Envelope` - ADSR-style envelopes
- `MouseModulator` - Map mouse position to parameters
- `AudioModulator` - (Placeholder for audio reactivity)
- `RandomWalk` - Smooth random parameter drift
- `ModMatrix` - Route multiple modulators to multiple parameters

### 🔌 **Patches** (`modules/patch.clj`)

Combine generators, effects, and modulators into signal chains:

```
Generator → Effect 1 → Effect 2 → ... → Renderer
              ↑          ↑
         Modulator 1  Modulator 2
```

## Quick Start

### Run with the original simple version:

```bash
lein run
```

### Run with modular architecture:

```bash
lein run -m fractal-explorer.core-modular
```

## Controls (Modular Version)

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

## Creating Custom Patches in the REPL

```clojure
(require '[.generators :as gen])
(require '[.effects :as fx])
(require '[.modulators :as mod])
(require '[.patch :as patch])

;; Create a custom patch
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

## Advanced Usage

### Chain Effects

Effects are applied in order, so you can create complex signal flows:

```clojure
(def complex-patch
  (patch/create-patch
    (gen/make-julia -0.8 0.156)

    ;; Color first, then distort, then add particles
    [(fx/make-color-mapper :ocean)
     (fx/make-ripple 0.05 15 2.0)
     (fx/make-echo 2 8 8 1.5)
     (fx/make-hue-shift 0)
     (fx/make-particles 0.85 0.015)]

    ;; LFO modulates hue shift
    [(mod/make-lfo 0.4 :sine)]

    {...}))
```

### Use Modulation Matrix

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

### Create New Generators

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

### Create New Effects

Implement the `Effect` protocol:

```clojure
(defrecord MyEffect [intensity]
  fx/Effect
  (process [this pixel-data params]
    ;; Transform pixel-data and return
    (map (fn [pixel]
           ;; Modify pixel values
           (update pixel :value * intensity))
         pixel-data)))
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

## Philosophy

This architecture lets you:

- **Experiment** - Try different combinations in the REPL
- **Compose** - Stack modules like Lego blocks
- **Extend** - Add new generators/effects/modulators
- **Share** - Save patches as data structures
- **Perform** - Live-code visual performances

Think of it as visual programming meets functional composition meets creative coding!
