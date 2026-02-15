(ns effects
  (:require [quil.core :as q]))

;; Protocol for all effects
(defprotocol Effect
  (process [this pixel-data params] "Process pixel data through effect"))

;; Color mapping effect
(defn color-map-fn
  [scheme]
  (case scheme
    :classic (fn [iter max-iter]
               (if (= iter max-iter)
                 [0 0 0]
                 (let [t (/ iter max-iter)]
                   [(* 255 (Math/sin (* t Math/PI)))
                    (* 255 (Math/sin (* t Math/PI 2)))
                    (* 255 (Math/cos (* t Math/PI)))])))

    :fire (fn [iter max-iter]
            (if (= iter max-iter)
              [0 0 0]
              (let [t (/ iter max-iter)]
                [(* 255 t)
                 (* 128 (* t t))
                 0])))

    :ocean (fn [iter max-iter]
             (if (= iter max-iter)
               [0 0 20]
               (let [t (/ iter max-iter)]
                 [0
                  (* 128 (+ 0.5 (* 0.5 (Math/sin (* t Math/PI 4)))))
                  (* 255 t)])))

    :psychedelic (fn [iter max-iter]
                   (if (= iter max-iter)
                     [0 0 0]
                     (let [t (/ iter max-iter)]
                       [(* 255 (Math/abs (Math/sin (* t Math/PI 3))))
                        (* 255 (Math/abs (Math/sin (* t Math/PI 5))))
                        (* 255 (Math/abs (Math/sin (* t Math/PI 7))))])))))

(defrecord ColorMapper [scheme]
  Effect
  (process [this pixel-data params]
    (let [color-fn (color-map-fn scheme)]
      (map (fn [pixel]
             (let [[r g b] (color-fn (:value pixel) (:max-value pixel))]
               (assoc pixel :r r :g g :b b)))
           pixel-data))))

;; Motion blur / trails effect
(defrecord MotionBlur [alpha]
  Effect
  (process [this pixel-data params]
    ;; This effect needs to be applied at render time
    ;; It modifies the background clear behavior
    (assoc params :motion-blur-alpha alpha)
    pixel-data))

;; Ripple distortion effect
(defrecord RippleDistortion [frequency amplitude speed]
  Effect
  (process [this pixel-data params]
    (let [time (:time params 0)]
      (map (fn [pixel]
             (let [x (:x pixel)
                   y (:y pixel)
                   dx (* amplitude (Math/sin (+ (* y frequency) (* time speed))))
                   dy (* amplitude (Math/sin (+ (* x frequency) (* time speed 1.5))))]
               (assoc pixel :x (+ x dx) :y (+ y dy))))
           pixel-data))))

;; Echo / multi-layer effect
(defrecord Echo [layers offset-x offset-y alpha-decay]
  Effect
  (process [this pixel-data params]
    (let [echoed (atom pixel-data)]
      (doseq [layer (range 1 (inc layers))]
        (let [echo-pixels (map (fn [pixel]
                                 (-> pixel
                                     (update :x + (* layer offset-x))
                                     (update :y + (* layer offset-y))
                                     (assoc :alpha (/ 1.0 (* (inc layer) alpha-decay)))))
                               pixel-data)]
          (swap! echoed concat echo-pixels)))
      @echoed)))

;; Feedback effect (blends with previous frame)
(defrecord Feedback [mix]
  Effect
  (process [this pixel-data params]
    ;; This effect needs state from previous frame
    ;; It's applied at render time
    (assoc params :feedback-mix mix)
    pixel-data))

;; Particle system effect
(defrecord ParticleSystem [threshold spawn-rate]
  Effect
  (process [this pixel-data params]
    (let [particles (:particles params [])
          new-particles (atom particles)]

      ;; Spawn new particles at high-value pixels
      (doseq [pixel pixel-data]
        (when (and (> (:value pixel) (* (:max-value pixel) threshold))
                   (< (rand) spawn-rate))
          (swap! new-particles conj
                 {:x (double (:x pixel))
                  :y (double (:y pixel))
                  :vx (- (rand 2) 1)
                  :vy (- (rand 2) 1)
                  :life 1.0
                  :r (or (:r pixel) 255)
                  :g (or (:g pixel) 255)
                  :b (or (:b pixel) 255)})))

      ;; Update existing particles
      (let [updated-particles (->> @new-particles
                                   (map (fn [p]
                                          (-> p
                                              (update :x + (:vx p))
                                              (update :y + (:vy p))
                                              (update :life - 0.02))))
                                   (filter #(> (:life %) 0))
                                   (take 500))]
        (assoc params :particles updated-particles)
        (concat pixel-data
                (map (fn [p]
                       {:x (:x p) :y (:y p)
                        :r (:r p) :g (:g p) :b (:b p)
                        :alpha (* 255 (:life p))
                        :size 3})
                     updated-particles))))))

;; Hue shift effect
(defrecord HueShift [amount]
  Effect
  (process [this pixel-data params]
    (map (fn [pixel]
           (if (and (:r pixel) (:g pixel) (:b pixel))
             (let [r (:r pixel)
                   g (:g pixel)
                   b (:b pixel)
                   ;; Simple hue rotation
                   shifted-r (mod (+ r amount) 255)
                   shifted-g (mod (+ g amount) 255)
                   shifted-b (mod (+ b amount) 255)]
               (assoc pixel :r shifted-r :g shifted-g :b shifted-b))
             pixel))
         pixel-data)))

;; Brightness/Contrast effect
(defrecord BrightnessContrast [brightness contrast]
  Effect
  (process [this pixel-data params]
    (map (fn [pixel]
           (if (and (:r pixel) (:g pixel) (:b pixel))
             (let [adjust (fn [c]
                            (-> c
                                (+ brightness)
                                (* contrast)
                                (max 0)
                                (min 255)))]
               (assoc pixel
                      :r (adjust (:r pixel))
                      :g (adjust (:g pixel))
                      :b (adjust (:b pixel))))
             pixel))
         pixel-data)))

;; Factory functions
(defn make-color-mapper [scheme] (->ColorMapper scheme))
(defn make-motion-blur [alpha] (->MotionBlur alpha))
(defn make-ripple [freq amp speed] (->RippleDistortion freq amp speed))
(defn make-echo [layers offset-x offset-y decay] (->Echo layers offset-x offset-y decay))
(defn make-feedback [mix] (->Feedback mix))
(defn make-particles [threshold rate] (->ParticleSystem threshold rate))
(defn make-hue-shift [amount] (->HueShift amount))
(defn make-brightness-contrast [brightness contrast] (->BrightnessContrast brightness contrast))
