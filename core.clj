(ns fractal-explorer.core
  (:require [quil.core :as q]
            [quil.middleware :as m]))

;; Fractal computation
(defn mandelbrot-iterations
  "Calculate iterations for a point in the complex plane.
   Returns number of iterations before escape (or max-iter)"
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

;; Color schemes
(def color-schemes
  {:classic (fn [iter max-iter]
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
                       (* 255 (Math/abs (Math/sin (* t Math/PI 7))))])))})

(def scheme-order [:classic :fire :ocean :psychedelic])

;; Visual effects
(defn apply-motion-blur
  "Blend current frame with previous frame for trails"
  [state]
  (when (:motion-blur state)
    (q/push-style)
    (q/fill 0 0 0 (* 255 0.15))  ; Adjust alpha for trail length
    (q/rect 0 0 (q/width) (q/height))
    (q/pop-style)))

(defn calculate-ripple-offset
  "Apply sinusoidal ripple distortion to coordinates"
  [state x y]
  (if (:ripple state)
    (let [time (/ (q/frame-count) 30.0)
          freq 0.05
          amp (* 0.3 (/ 1.0 (:zoom state)))
          dx (* amp (Math/sin (+ (* y freq) (* time 2))))
          dy (* amp (Math/sin (+ (* x freq) (* time 1.5))))]
      [(+ x dx) (+ y dy)])
    [x y]))

(defn spawn-particles
  "Generate particles at high-iteration points"
  [state iter max-iter px py]
  (if (and (:particles state)
           (> iter (* max-iter 0.8))
           (< (rand) 0.02))
    (update state :particle-list conj
            {:x (double px)
             :y (double py)
             :vx (- (rand 2) 1)
             :vy (- (rand 2) 1)
             :life 1.0
             :color [(rand-int 255) (rand-int 255) (rand-int 255)]})
    state))

(defn update-particles
  "Update particle positions and lifetimes"
  [state]
  (if (:particles state)
    (assoc state :particle-list
           (->> (:particle-list state)
                (map (fn [p]
                       (-> p
                           (update :x + (:vx p))
                           (update :y + (:vy p))
                           (update :life - 0.02))))
                (filter #(> (:life %) 0))
                (take 500)))  ; Limit particle count
    state))

(defn draw-particles
  "Render active particles"
  [state]
  (when (:particles state)
    (doseq [{:keys [x y life color]} (:particle-list state)]
      (q/push-style)
      (q/no-stroke)
      (q/fill (first color) (second color) (nth color 2) (* 255 life))
      (q/ellipse x y 3 3)
      (q/pop-style))))

;; State management
(defn setup []
  (q/frame-rate 30)
  {:center-x -0.5
   :center-y 0.0
   :zoom 1.0
   :max-iter 100
   :color-scheme :classic
   :scheme-index 0
   :width (q/width)
   :height (q/height)
   ;; Visual effects toggles
   :motion-blur false
   :fractal-echo false
   :ripple false
   :particles false
   :feedback false
   :particle-list []
   :echo-layers 3
   :feedback-buffer nil})

(defn screen-to-complex
  "Convert screen coordinates to complex plane coordinates"
  [state x y]
  (let [[rx ry] (calculate-ripple-offset state x y)
        aspect (/ (:width state) (:height state))
        scale (/ 4.0 (:zoom state))
        cx (+ (:center-x state) (* (- (/ rx (:width state)) 0.5) scale aspect))
        cy (+ (:center-y state) (* (- (/ ry (:height state)) 0.5) scale))]
    [cx cy]))

(defn draw-state [state]
  ;; Apply motion blur by drawing semi-transparent black rectangle
  (if (:motion-blur state)
    (apply-motion-blur state)
    (q/background 0))
  
  ;; Apply feedback effect - blend with previous frame
  (when (and (:feedback state) (:feedback-buffer state))
    (q/push-style)
    (q/tint 255 230)  ; Slight transparency
    (q/image (:feedback-buffer state) 0 0)
    (q/pop-style))
  
  (let [w (:width state)
        h (:height state)
        max-iter (:max-iter state)
        scheme-fn (get color-schemes (:color-scheme state))
        state-ref (atom state)]  ; For particle spawning
    
    ;; Draw main fractal
    (doseq [px (range 0 w 2)
            py (range 0 h 2)]
      (let [[cx cy] (screen-to-complex @state-ref px py)
            iter (mandelbrot-iterations cx cy max-iter)
            [r g b] (scheme-fn iter max-iter)]
        (q/fill r g b)
        (q/no-stroke)
        (q/rect px py 2 2)
        
        ;; Spawn particles at interesting points
        (swap! state-ref spawn-particles iter max-iter px py)))
    
    ;; Draw fractal echo layers
    (when (:fractal-echo state)
      (q/push-style)
      (dotimes [layer (:echo-layers state)]
        (let [offset (* (inc layer) 5)
              scale-factor (- 1.0 (* layer 0.1))
              alpha (/ 100 (inc layer))]
          (doseq [px (range 0 w 4)
                  py (range 0 h 4)]
            (let [scaled-x (+ (* (- px (/ w 2)) scale-factor) (/ w 2) offset)
                  scaled-y (+ (* (- py (/ h 2)) scale-factor) (/ h 2) offset)
                  [cx cy] (screen-to-complex state scaled-x scaled-y)
                  iter (mandelbrot-iterations cx cy max-iter)
                  [r g b] (scheme-fn iter max-iter)]
              (q/fill r g b alpha)
              (q/no-stroke)
              (q/rect px py 4 4)))))
      (q/pop-style))
    
    ;; Draw particles
    (draw-particles @state-ref)
    
    ;; Store current frame for feedback effect
    (let [new-state (if (:feedback state)
                      (assoc @state-ref :feedback-buffer (q/get-pixel 0 0 w h))
                      @state-ref)]
      
      ;; Display info
      (q/fill 255 255 255 200)
      (q/text-size 12)
      (q/text (str "Zoom: " (format "%.2f" (:zoom state)) "x | "
                   "Iterations: " (:max-iter state) " | "
                   "Color: " (name (:color-scheme state)) "\n"
                   "Effects: "
                   (when (:motion-blur state) "BLUR ")
                   (when (:fractal-echo state) "ECHO ")
                   (when (:ripple state) "RIPPLE ")
                   (when (:particles state) "PARTICLES ")
                   (when (:feedback state) "FEEDBACK ")
                   "\n"
                   "Click=zoom | Space=color | +/-=detail | 1-5=toggle effects | R=reset")
              10 20)
      
      ;; Return updated state with new particle list
      new-state)))

(defn mouse-clicked [state event]
  (let [[cx cy] (screen-to-complex state (:x event) (:y event))]
    (-> state
        (assoc :center-x cx)
        (assoc :center-y cy)
        (update :zoom * 2.0))))

(defn key-pressed [state event]
  (case (:key event)
    :space (let [new-index (mod (inc (:scheme-index state)) (count scheme-order))]
             (-> state
                 (assoc :scheme-index new-index)
                 (assoc :color-scheme (nth scheme-order new-index))))
    
    (\+ \=) (update state :max-iter #(min 500 (+ % 20)))
    (\- \_) (update state :max-iter #(max 20 (- % 20)))
    
    :r (setup)  ; Reset
    
    state))

(defn -main [& args]
  (q/defsketch fractal-explorer
    :title "Fractal Explorer - Click to zoom, Space for colors, +/- for detail"
    :size [800 600]
    :setup setup
    :draw draw-state
    :mouse-clicked mouse-clicked
    :key-pressed key-pressed
    :middleware [m/fun-mode]))
