(ns modulators-test
  (:require [clojure.test :refer [deftest is testing]]
            [modulators :as mod]))

;; --- map-range ---

(deftest map-range-test
  (testing "maps minimum input to minimum output"
    (is (= 0.0 (mod/map-range -1 -1 1 0 100))))
  (testing "maps maximum input to maximum output"
    (is (= 100.0 (mod/map-range 1 -1 1 0 100))))
  (testing "maps midpoint correctly"
    (is (= 50.0 (mod/map-range 0 -1 1 0 100))))
  (testing "works with non-standard input ranges"
    (is (= 50.0 (mod/map-range 5 0 10 0 100)))))

;; --- LFO ---

(deftest lfo-test
  (testing "sine at time=0 phase=0 is 0"
    (let [lfo (mod/make-lfo 1.0 0 :sine)]
      (is (< (Math/abs (mod/modulate lfo {:time 0})) 1e-10))))

  (testing "sine output stays in [-1, 1]"
    (let [lfo (mod/make-lfo 0.5 :sine)]
      (doseq [t (range 0 20)]
        (is (<= -1.0 (mod/modulate lfo {:time t}) 1.0)))))

  (testing "triangle output stays in [-1, 1]"
    (let [lfo (mod/make-lfo 1.0 0 :triangle)]
      (doseq [t (range 0 20)]
        (is (<= -1.0 (mod/modulate lfo {:time t}) 1.0)))))

  (testing "square wave is exactly 1.0 or -1.0"
    (let [lfo (mod/make-lfo 1.0 0 :square)]
      (doseq [t [0.1 0.5 1.1 1.5 2.1]]
        (is (contains? #{1.0 -1.0} (mod/modulate lfo {:time t}))))))

  (testing "saw output stays in [-1, 1]"
    (let [lfo (mod/make-lfo 1.0 0 :saw)]
      (doseq [t (range 0 20)]
        (is (<= -1.0 (mod/modulate lfo {:time t}) 1.0)))))

  (testing "frequency affects period — two LFOs at different freqs differ"
    (let [slow (mod/make-lfo 0.1 :sine)
          fast (mod/make-lfo 2.0 :sine)]
      (is (not= (mod/modulate slow {:time 1.0})
                (mod/modulate fast {:time 1.0}))))))

;; --- Envelope ---

(deftest envelope-test
  (let [env (mod/make-envelope 1.0 0.5 0.7 0.5)]
    (testing "starts at 0 at trigger time"
      (is (= 0.0 (mod/modulate env {:time 0 :trigger-time 0}))))

    (testing "reaches peak (1.0) at end of attack"
      (is (= 1.0 (mod/modulate env {:time 1.0 :trigger-time 0}))))

    (testing "settles to sustain level after decay completes"
      (is (= 0.7 (mod/modulate env {:time 10.0 :trigger-time 0}))))

    (testing "result is bounded between 0 and 1"
      (doseq [t [0 0.5 1.0 1.25 10.0]]
        (is (<= 0.0 (mod/modulate env {:time t :trigger-time 0}) 1.0))))))

;; --- ModMatrix ---

;; A deterministic modulator that always returns a fixed scalar value.
(defn- fixed-mod [value]
  (reify mod/Modulator
    (modulate [_ _params] value)))

(deftest mod-matrix-test
  (testing "maps max modulator output (1.0) to :max of target range"
    (let [matrix (mod/make-mod-matrix [{:modulator (fixed-mod 1.0)
                                        :param :zoom
                                        :min 1.0
                                        :max 5.0}])
          result (mod/modulate matrix {:zoom 1.0 :time 0})]
      (is (= 5.0 (:zoom result)))))

  (testing "maps min modulator output (-1.0) to :min of target range"
    (let [matrix (mod/make-mod-matrix [{:modulator (fixed-mod -1.0)
                                        :param :zoom
                                        :min 1.0
                                        :max 5.0}])
          result (mod/modulate matrix {:zoom 3.0 :time 0})]
      (is (= 1.0 (:zoom result)))))

  (testing "maps midpoint (0.0) to midpoint of target range"
    (let [matrix (mod/make-mod-matrix [{:modulator (fixed-mod 0.0)
                                        :param :zoom
                                        :min 1.0
                                        :max 5.0}])
          result (mod/modulate matrix {:zoom 1.0 :time 0})]
      (is (= 3.0 (:zoom result)))))

  (testing "routes multiple mappings independently"
    (let [matrix (mod/make-mod-matrix [{:modulator (fixed-mod 0.0) :param :zoom     :min 1.0 :max 5.0}
                                       {:modulator (fixed-mod 0.0) :param :max-iter :min 50  :max 200}])
          result (mod/modulate matrix {:zoom 1.0 :max-iter 100 :time 0})]
      (is (= 3.0   (:zoom result)))
      (is (= 125.0 (:max-iter result)))))

  (testing "does not modify unrelated params"
    (let [matrix (mod/make-mod-matrix [{:modulator (fixed-mod 1.0) :param :zoom :min 1.0 :max 5.0}])
          result (mod/modulate matrix {:zoom 1.0 :center-x -0.5 :time 0})]
      (is (= -0.5 (:center-x result))))))
