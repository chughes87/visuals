;; Core
(ns core
  (:require
   [patch :as patch]
   [presets :as presets :refer [load-preset]]
   [quil.core :as q]
   [quil.middleware :as m] ;; [generators :as gen]
   ))

;; Rendering
(defn render-pixels
  "Render pixel data to screen"
  [pixel-data params]
  ;; Apply motion blur if enabled (single rect call, not per-pixel)
  (when-let [blur-alpha (:motion-blur-alpha params)]
    (q/push-style)
    (q/fill 0 0 0 (* 255 blur-alpha))
    (q/rect 0 0 (q/width) (q/height))
    (q/pop-style))

  ;; Write directly to Processing's pixel buffer instead of individual
  ;; rect calls — eliminates ~5 API calls per pixel (push-style, no-stroke,
  ;; fill, rect, pop-style) and replaces them with a single array write.
  (let [w      (q/width)
        h      (q/height)
        applet (q/current-applet)]
    (.loadPixels applet)
    (let [^ints px-buf (.-pixels applet)]
      (doseq [pixel pixel-data]
        (when (and (:x pixel) (:y pixel))
          (let [px   (int (:x pixel))
                py   (int (:y pixel))
                r    (int (:r pixel 255))
                g    (int (:g pixel 255))
                b    (int (:b pixel 255))
                size (int (:size pixel 2))]
            (if-let [raw-alpha (:alpha pixel)]
              ;; Translucent pixel: blend over whatever is already in the buffer
              (let [a     (int raw-alpha)
                    a-f   (/ a 255.0)
                    a-inv (- 1.0 a-f)]
                (dotimes [dy size]
                  (dotimes [dx size]
                    (let [nx (unchecked-add px dx)
                          ny (unchecked-add py dy)]
                      (when (and (< -1 nx w) (< -1 ny h))
                        (let [idx (unchecked-add (unchecked-multiply ny w) nx)
                              cur (aget px-buf idx)
                              cr  (bit-and (bit-shift-right cur 16) 0xFF)
                              cg  (bit-and (bit-shift-right cur 8)  0xFF)
                              cb  (bit-and cur 0xFF)]
                          (aset px-buf idx
                                (unchecked-int
                                  (bit-or 0xFF000000
                                          (bit-shift-left (int (+ (* r a-f) (* cr a-inv))) 16)
                                          (bit-shift-left (int (+ (* g a-f) (* cg a-inv))) 8)
                                          (int (+ (* b a-f) (* cb a-inv))))))))))))))
              ;; Opaque pixel: write directly — fast path for the common case
              (let [argb (unchecked-int (bit-or 0xFF000000
                                                (bit-shift-left r 16)
                                                (bit-shift-left g 8)
                                                b))]
                (dotimes [dy size]
                  (dotimes [dx size]
                    (let [nx (unchecked-add px dx)
                          ny (unchecked-add py dy)]
                      (when (and (< -1 nx w) (< -1 ny h))
                        (aset px-buf (unchecked-add (unchecked-multiply ny w) nx)
                              argb))))))))))
    (.updatePixels applet)))

(def ^:private spinner-frames ["⠋" "⠙" "⠹" "⠸" "⠼" "⠴" "⠦" "⠧" "⠇" "⠏"])

(defn draw-ui
  "Draw UI overlay with patch info"
  [state]
  (q/push-style)
  (q/fill 255 255 255 200)
  (q/text-size 12)
  (let [patch (:patch state)
        params (:params patch)]
    (q/text (str "Preset: " (name (:current-preset state)) "\n"
                 "Zoom: " (format "%.2f" (:zoom params 1.0)) "x | "
                 "Iterations: " (:max-iter params 100) "\n"
                 "Effects: " (count (:effects patch)) " active\n"
                 "Modulators: " (count (:modulators patch)) " active\n"
                 "\n"
                 "Click=zoom | 1-5=presets | Space=next preset\n"
                 "+/-=iterations | R=reset | Q=quit")
            10 20))
  ;; Render status indicator at bottom-left
  (let [frame              (:frame state)
        recomputed-at      (:recomputed-at-frame state -1)
        frames-since       (- frame recomputed-at)
        flash-duration     45   ; frames (~1.5 s) to show the refresh flash
        spinning?          (< frames-since flash-duration)]
    (if spinning?
      (let [spinner (nth spinner-frames (mod frame (count spinner-frames)))]
        (q/fill 255 220 80 230)
        (q/text (str spinner " refreshed") 10 (- (q/height) 12)))
      (do
        (q/fill 100 220 100 180)
        (q/text "● cached" 10 (- (q/height) 12)))))
  (q/pop-style))

;; Setup
(defn setup []
  (q/frame-rate 30)
  (q/color-mode :rgb)
  (let [w (q/width)
        h (q/height)]
    {:patch (presets/classic-mandelbrot-patch w h)
     :current-preset :classic-mandelbrot
     :width w
     :height h
     :frame 0}))

;; Update
(defn update-state [state]
  (let [;; Increment time for modulators
        patch (patch/update-patch-params
               (:patch state)
               {:time (/ (:frame state) 30.0)})

        ;; Process the patch
        result (patch/process-patch patch)

        ;; Update state - use the patch returned by process-patch so the
        ;; generator cache persists into the next frame
        new-state (-> state
                      (assoc :patch (:patch result))
                      (assoc :pixel-data (:pixel-data result))
                      (assoc :cache-hit? (:cache-hit? result))
                      (update :recomputed-at-frame
                              (fn [prev] (if (:cache-hit? result) prev (:frame state))))
                      (update :frame inc))]
    new-state))

;; Draw
(defn draw-state [state]
  (q/background 0)

  ;; Render the processed pixels
  (render-pixels (:pixel-data state)
                 (get-in state [:patch :params]))

  ;; Draw UI
  (draw-ui state))

;; Mouse interaction
(defn mouse-clicked [state event]
  (let [patch (:patch state)
        params (:params patch)
        w (:width state)
        h (:height state)
        aspect (/ w h)
        scale (/ 4.0 (:zoom params 1.0))
        cx (:center-x params 0)
        cy (:center-y params 0)
        new-cx (+ cx (* (- (/ (:x event) w) 0.5) scale aspect))
        new-cy (+ cy (* (- (/ (:y event) h) 0.5) scale))]
    (-> state
        (assoc :patch
               (patch/update-patch-params
                patch
                {:center-x new-cx
                 :center-y new-cy
                 :zoom (* (:zoom params 1.0) 2.0)})))))


;; Keyboard interaction


(defn key-pressed [state event]
  (let [patch (:patch state)]
    (case (:key event)
      ;; Preset switching
      \1 (-> state
             (assoc :patch (load-preset :classic-mandelbrot (:width state) (:height state)))
             (assoc :current-preset :classic-mandelbrot))

      \2 (-> state
             (assoc :patch (load-preset :psychedelic-julia (:width state) (:height state)))
             (assoc :current-preset :psychedelic-julia))

      \3 (-> state
             (assoc :patch (load-preset :trippy-mandelbrot (:width state) (:height state)))
             (assoc :current-preset :trippy-mandelbrot))

      \4 (-> state
             (assoc :patch (load-preset :burning-ship-trails (:width state) (:height state)))
             (assoc :current-preset :burning-ship-trails))

      \5 (-> state
             (assoc :patch (load-preset :noise-field (:width state) (:height state)))
             (assoc :current-preset :noise-field))

      ;; Cycle through presets
      :space (let [presets (keys presets/preset-patches)
                   current-idx (.indexOf (vec presets) (:current-preset state))
                   next-idx (mod (inc current-idx) (count presets))
                   next-preset (nth (vec presets) next-idx)]
               (-> state
                   (assoc :patch (load-preset next-preset (:width state) (:height state)))
                   (assoc :current-preset next-preset)))

      ;; Iteration control
      (\+ \=) (assoc state :patch
                     (patch/update-patch-params patch
                                                {:max-iter (min 500 (+ (:max-iter (:params patch) 100) 20))}))

      (\- \_) (assoc state :patch
                     (patch/update-patch-params patch
                                                {:max-iter (max 20 (- (:max-iter (:params patch) 100) 20))}))

      ;; Reset
      :r (setup)

      ;; Default
      state)))

;; Main
(defn -main [& args]
  (q/defsketch core
    :title "Fractal Explorer - Modular Visual Synthesizer"
    :size [800 600]
    :setup setup
    :update update-state
    :draw draw-state
    :mouse-clicked mouse-clicked
    :key-pressed key-pressed
    :middleware [m/fun-mode]))