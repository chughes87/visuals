(ns fractal-explorer.modules.modulators
  (:require [quil.core :as q]))

;; Protocol for all modulators
(defprotocol Modulator
  (modulate [this params] "Return modulated value based on current state"))

;; Low Frequency Oscillator (LFO)
(defrecord LFO [frequency phase waveform]
  Modulator
  (modulate [this params]
    (let [time (:time params 0)
          t (+ (* time frequency) phase)]
      (case waveform
        :sine (Math/sin t)
        :triangle (- (* 2 (Math/abs (- (mod t (* 2 Math/PI)) Math/PI))) 1)
        :square (if (< (mod t (* 2 Math/PI)) Math/PI) 1.0 -1.0)
        :saw (- (* 2 (/ (mod t (* 2 Math/PI)) (* 2 Math/PI))) 1)
        (Math/sin t)))))

;; Envelope generator (ADSR style)
(defrecord Envelope [attack decay sustain release]
  Modulator
  (modulate [this params]
    (let [trigger-time (:trigger-time params 0)
          current-time (:time params 0)
          elapsed (- current-time trigger-time)]
      (cond
        (< elapsed attack) (/ elapsed attack)
        (< elapsed (+ attack decay)) (+ sustain (* (- 1 sustain) 
                                                    (- 1 (/ (- elapsed attack) decay))))
        :else sustain))))

;; Mouse position modulator
(defrecord MouseModulator [axis scale offset]
  Modulator
  (modulate [this params]
    (let [mouse-val (case axis
                      :x (q/mouse-x)
                      :y (q/mouse-y)
                      0)
          screen-size (case axis
                        :x (:width params 800)
                        :y (:height params 600)
                        1)]
      (+ offset (* scale (/ mouse-val screen-size))))))

;; Audio-reactive modulator (placeholder - could connect to sound input)
(defrecord AudioModulator [frequency-band sensitivity]
  Modulator
  (modulate [this params]
    ;; Placeholder - simulate audio reactivity with noise
    (* sensitivity (q/noise (* (q/frame-count) 0.1) frequency-band))))

;; Random walk modulator
(defrecord RandomWalk [step-size smoothing]
  Modulator
  (modulate [this params]
    (let [prev-value (:random-walk-value params 0)
          random-step (* step-size (- (rand 2) 1))
          new-value (+ prev-value random-step)
          smoothed (+ (* smoothing prev-value) (* (- 1 smoothing) new-value))]
      (assoc params :random-walk-value smoothed)
      smoothed)))

;; Parameter mapper - maps modulator output to parameter range
(defn map-range
  "Map value from input range to output range"
  [value in-min in-max out-min out-max]
  (+ out-min (* (- out-max out-min) 
                (/ (- value in-min) (- in-max in-min)))))

(defn apply-modulator
  "Apply modulator to a parameter in the params map"
  [params param-key modulator out-min out-max]
  (let [mod-value (modulate modulator params)
        mapped-value (map-range mod-value -1.0 1.0 out-min out-max)]
    (assoc params param-key mapped-value)))

;; Modulation matrix - route multiple modulators to multiple parameters
(defrecord ModMatrix [mappings]
  ; mappings: [{:modulator mod :param :zoom :min 1.0 :max 10.0} ...]
  Modulator
  (modulate [this params]
    (reduce (fn [p mapping]
              (apply-modulator p 
                             (:param mapping)
                             (:modulator mapping)
                             (:min mapping)
                             (:max mapping)))
            params
            mappings)))

;; Factory functions
(defn make-lfo 
  ([frequency] (->LFO frequency 0 :sine))
  ([frequency waveform] (->LFO frequency 0 waveform))
  ([frequency phase waveform] (->LFO frequency phase waveform)))

(defn make-envelope [attack decay sustain release]
  (->Envelope attack decay sustain release))

(defn make-mouse-mod 
  ([axis] (->MouseModulator axis 1.0 0.0))
  ([axis scale offset] (->MouseModulator axis scale offset)))

(defn make-audio-mod [freq-band sensitivity]
  (->AudioModulator freq-band sensitivity))

(defn make-random-walk [step-size smoothing]
  (->RandomWalk step-size smoothing))

(defn make-mod-matrix [mappings]
  (->ModMatrix mappings))
