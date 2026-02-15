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
   :height (q/height)})

(defn screen-to-complex
  "Convert screen coordinates to complex plane coordinates"
  [state x y]
  (let [aspect (/ (:width state) (:height state))
        scale (/ 4.0 (:zoom state))
        cx (+ (:center-x state) (* (- (/ x (:width state)) 0.5) scale aspect))
        cy (+ (:center-y state) (* (- (/ y (:height state)) 0.5) scale))]
    [cx cy]))

(defn draw-state [state]
  (q/background 0)
  
  (let [w (:width state)
        h (:height state)
        max-iter (:max-iter state)
        scheme-fn (get color-schemes (:color-scheme state))]
    
    (doseq [px (range 0 w 2)  ; Render every 2 pixels for speed
            py (range 0 h 2)]
      (let [[cx cy] (screen-to-complex state px py)
            iter (mandelbrot-iterations cx cy max-iter)
            [r g b] (scheme-fn iter max-iter)]
        (q/fill r g b)
        (q/no-stroke)
        (q/rect px py 2 2))))
  
  ;; Display info
  (q/fill 255 255 255 200)
  (q/text-size 14)
  (q/text (str "Zoom: " (format "%.2f" (:zoom state)) "x\n"
               "Max iterations: " (:max-iter state) "\n"
               "Color: " (name (:color-scheme state)) "\n"
               "Click to zoom in | Space: cycle colors | +/-: iterations")
          10 20))

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
