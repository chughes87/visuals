(ns fractal-explorer.modules.patch
  (:require [fractal-explorer.modules.generators :as gen]
            [fractal-explorer.modules.effects :as fx]
            [fractal-explorer.modules.modulators :as mod]))

;; A patch is a signal flow graph
;; generator -> [effects] -> renderer

(defrecord Patch [generator effects modulators params])

(defn create-patch
  "Create a new patch with generator, effect chain, and modulators"
  [generator effects modulators initial-params]
  (->Patch generator effects modulators initial-params))

(defn process-patch
  "Process a patch: apply modulators, generate signal, apply effects"
  [patch]
  (let [;; First apply all modulators to update parameters
        modulated-params (reduce (fn [p modulator]
                                  (mod/modulate modulator p))
                                (:params patch)
                                (:modulators patch))
        
        ;; Generate base signal
        pixel-data (gen/generate (:generator patch) modulated-params)
        
        ;; Apply effect chain
        processed-data (reduce (fn [data effect]
                                (fx/process effect data modulated-params))
                              pixel-data
                              (:effects patch))]
    
    {:pixel-data processed-data
     :params modulated-params}))

(defn update-patch-params
  "Update patch parameters (for user interaction)"
  [patch updates]
  (update patch :params merge updates))

(defn add-effect
  "Add an effect to the end of the effect chain"
  [patch effect]
  (update patch :effects conj effect))

(defn remove-effect
  "Remove an effect at index from the chain"
  [patch index]
  (update patch :effects 
          (fn [effects] 
            (vec (concat (take index effects) 
                        (drop (inc index) effects))))))

(defn replace-effect
  "Replace effect at index"
  [patch index new-effect]
  (update patch :effects assoc index new-effect))

(defn add-modulator
  "Add a modulator to the patch"
  [patch modulator]
  (update patch :modulators conj modulator))

(defn replace-generator
  "Swap out the generator"
  [patch new-generator]
  (assoc patch :generator new-generator))

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
    [(mod/make-lfo 0.5 :sine)]  ; Modulates hue shift
    {:width width
     :height height
     :center-x 0.0
     :center-y 0.0
     :zoom 1.0
     :max-iter 100
     :time 0}))

(defn trippy-mandelbrot-patch
  [width height]
  (create-patch
    (gen/make-mandelbrot)
    [(fx/make-color-mapper :ocean)
     (fx/make-ripple 0.05 10 2)
     (fx/make-echo 3 5 5 2.0)
     (fx/make-particles 0.8 0.01)]
    [(mod/make-lfo 0.3 :sine)]
    {:width width
     :height height
     :center-x -0.5
     :center-y 0.0
     :zoom 1.0
     :max-iter 100
     :time 0
     :particles []}))

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
    [(mod/make-lfo 0.2 :sine)]
    {:width width
     :height height
     :time 0}))

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
    (classic-mandelbrot-patch width height)))
