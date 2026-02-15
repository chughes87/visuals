(ns fractal-explorer.modules.generators
  (:require [quil.core :as q]))

;; Protocol for all generators
(defprotocol Generator
  (generate [this params] "Generate visual data based on parameters"))

;; Mandelbrot generator
(defn mandelbrot-iterations
  [cx cy max-iter]
  (loop [x 0.0
         y 0.0
         iter 0]
    (let [x2 (* x x)
          y2 (* y y)]
      (if (or (>= iter max-iter)
              (> (+ x2 y2) 4.0))
        iter
        (recur (+ (- x2 y2) cx)
               (+ (* 2 x y) cy)
               (inc iter))))))

(defrecord MandelbrotGenerator []
  Generator
  (generate [this params]
    (let [{:keys [width height center-x center-y zoom max-iter]} params
          aspect (/ width height)
          scale (/ 4.0 zoom)
          pixel-data (atom [])]
      (doseq [py (range 0 height 2)
              px (range 0 width 2)]
        (let [cx (+ center-x (* (- (/ px width) 0.5) scale aspect))
              cy (+ center-y (* (- (/ py height) 0.5) scale))
              iter (mandelbrot-iterations cx cy max-iter)]
          (swap! pixel-data conj {:x px :y py :value iter :max-value max-iter})))
      @pixel-data)))

;; Julia set generator
(defn julia-iterations
  [zx zy c-real c-imag max-iter]
  (loop [x zx
         y zy
         iter 0]
    (let [x2 (* x x)
          y2 (* y y)]
      (if (or (>= iter max-iter)
              (> (+ x2 y2) 4.0))
        iter
        (recur (+ (- x2 y2) c-real)
               (+ (* 2 x y) c-imag)
               (inc iter))))))

(defrecord JuliaGenerator [c-real c-imag]
  Generator
  (generate [this params]
    (let [{:keys [width height center-x center-y zoom max-iter]} params
          aspect (/ width height)
          scale (/ 4.0 zoom)
          pixel-data (atom [])]
      (doseq [py (range 0 height 2)
              px (range 0 width 2)]
        (let [zx (+ center-x (* (- (/ px width) 0.5) scale aspect))
              zy (+ center-y (* (- (/ py height) 0.5) scale))
              iter (julia-iterations zx zy c-real c-imag max-iter)]
          (swap! pixel-data conj {:x px :y py :value iter :max-value max-iter})))
      @pixel-data)))

;; Perlin noise generator
(defrecord NoiseGenerator [scale octaves]
  Generator
  (generate [this params]
    (let [{:keys [width height time]} params
          pixel-data (atom [])]
      (doseq [py (range 0 height 4)
              px (range 0 width 4)]
        (let [noise-val (q/noise (* px scale) (* py scale) time)
              value (* noise-val 100)]
          (swap! pixel-data conj {:x px :y py :value value :max-value 100})))
      @pixel-data)))

;; Burning Ship fractal
(defn burning-ship-iterations
  [cx cy max-iter]
  (loop [x 0.0
         y 0.0
         iter 0]
    (let [x2 (* x x)
          y2 (* y y)]
      (if (or (>= iter max-iter)
              (> (+ x2 y2) 4.0))
        iter
        (recur (+ (- x2 y2) cx)
               (+ (* 2 (Math/abs x) (Math/abs y)) cy)
               (inc iter))))))

(defrecord BurningShipGenerator []
  Generator
  (generate [this params]
    (let [{:keys [width height center-x center-y zoom max-iter]} params
          aspect (/ width height)
          scale (/ 4.0 zoom)
          pixel-data (atom [])]
      (doseq [py (range 0 height 2)
              px (range 0 width 2)]
        (let [cx (+ center-x (* (- (/ px width) 0.5) scale aspect))
              cy (+ center-y (* (- (/ py height) 0.5) scale))
              iter (burning-ship-iterations cx cy max-iter)]
          (swap! pixel-data conj {:x px :y py :value iter :max-value max-iter})))
      @pixel-data)))

;; Factory functions
(defn make-mandelbrot [] (->MandelbrotGenerator))
(defn make-julia [c-real c-imag] (->JuliaGenerator c-real c-imag))
(defn make-noise [scale octaves] (->NoiseGenerator scale octaves))
(defn make-burning-ship [] (->BurningShipGenerator))
