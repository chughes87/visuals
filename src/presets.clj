(ns presets
  (:require
   [effects :as fx]
   [generators :as gen]
   [modulators :as mod]
   [patch :refer [create-patch]]))

;; Preset patches
(defn classic-mandelbrot-patch
  [width height]
  (create-patch
   (gen/make-mandelbrot)
   [(fx/make-color-mapper :classic)]
   []
   {:width width
    :height height
    :center-x -0.5
    :center-y 0.0
    :zoom 1.0
    :max-iter 100
    :time 0}))

(defn psychedelic-julia-patch
  [width height]
  (create-patch
   (gen/make-julia -0.7 0.27015)
   [(fx/make-color-mapper :psychedelic)
    (fx/make-hue-shift 0)]
    ;; Use ModMatrix to route LFO to hue-shift parameter
   [(mod/make-mod-matrix
     [{:modulator (mod/make-lfo 0.5 :sine)
       :param :hue-shift-amount
       :min 0
       :max 255}])]
   {:width width
    :height height
    :center-x 0.0
    :center-y 0.0
    :zoom 1.0
    :max-iter 100
    :time 0
    :hue-shift-amount 0}))

(defn burning-ship-trails-patch
  [width height]
  (create-patch
   (gen/make-burning-ship)
   [(fx/make-color-mapper :fire)
    (fx/make-motion-blur 0.15)]
   []
   {:width width
    :height height
    :center-x -0.5
    :center-y -0.5
    :zoom 1.0
    :max-iter 100
    :time 0}))

(defn noise-field-patch
  [width height]
  (create-patch
   (gen/make-noise 0.01 4)
   [(fx/make-color-mapper :psychedelic)
    (fx/make-brightness-contrast 20 1.5)]
    ;; Use ModMatrix to route LFO to brightness
   [(mod/make-mod-matrix
     [{:modulator (mod/make-lfo 0.2 :sine)
       :param :brightness-amount
       :min 0
       :max 40}])]
   {:width width
    :height height
    :time 0
    :brightness-amount 20}))

(defn trippy-mandelbrot-patch
  [width height]
  (create-patch
   (gen/make-mandelbrot)
   [(fx/make-color-mapper :ocean)
    (fx/make-ripple 0.05 10 2)
    (fx/make-echo 3 5 5 2.0)
    (fx/make-particles 0.8 0.01)]
    ;; Use ModMatrix to route LFO to ripple amplitude
   [(mod/make-mod-matrix
     [{:modulator (mod/make-lfo 0.3 :sine)
       :param :ripple-amplitude
       :min 5
       :max 15}])]
   {:width width
    :height height
    :center-x -0.5
    :center-y 0.0
    :zoom 1.0
    :max-iter 100
    :time 0
    :particles []
    :ripple-amplitude 10}))

;; Patch library
(def preset-patches
  {:classic-mandelbrot classic-mandelbrot-patch
   :psychedelic-julia psychedelic-julia-patch
   :trippy-mandelbrot trippy-mandelbrot-patch
   :burning-ship-trails burning-ship-trails-patch
   :noise-field noise-field-patch})

(defn load-preset
  "Load a preset patch by name"
  [preset-name width height]
  (if-let [patch-fn (get preset-patches preset-name)]
    (patch-fn width height)
    (classic-mandelbrot-patch width height))) (defn preset-patches [arg1])
