(ns effects-test
  (:require [clojure.test :refer [deftest is testing]]
            [effects :as fx]))

(def sample-params {:width 800 :height 600 :time 0})

(def plain-pixels
  [{:x 10 :y 20 :value 50 :max-value 100}
   {:x 30 :y 40 :value 100 :max-value 100}])

(def colored-pixels
  [{:x 10 :y 20 :value 50 :max-value 100 :r 100 :g 150 :b 200}
   {:x 30 :y 40 :value 25 :max-value 100 :r 50  :g 80  :b 20}])

;; --- ColorMapper ---

(deftest color-mapper-test
  (testing "assigns :r :g :b to all pixels"
    (let [result (fx/process (fx/make-color-mapper :classic) plain-pixels sample-params)]
      (is (every? #(and (contains? % :r) (contains? % :g) (contains? % :b)) result))))

  (testing "interior points (value = max-value) render black in classic scheme"
    (let [interior [{:x 0 :y 0 :value 100 :max-value 100}]
          result   (first (fx/process (fx/make-color-mapper :classic) interior sample-params))]
      (is (= 0 (:r result) (:g result) (:b result)))))

  (testing "all built-in schemes produce output"
    (doseq [scheme [:classic :fire :ocean :psychedelic]]
      (let [result (fx/process (fx/make-color-mapper scheme) plain-pixels sample-params)]
        (is (every? #(contains? % :r) result)
            (str "scheme " scheme " should assign :r"))))))

;; --- RippleDistortion ---

(deftest ripple-distortion-test
  (testing "shifts pixel coordinates when amplitude > 0"
    (let [pixel  [{:x 100 :y 100 :value 50 :max-value 100}]
          result (first (fx/process (fx/make-ripple 0.1 10 1.0) pixel {:time 0}))]
      ;; dx = 10*sin(100*0.1 + 0) ≠ 0 for this input
      (is (not= 100 (:x result)))))

  (testing "zero amplitude causes no displacement"
    (let [pixel  [{:x 100 :y 100 :value 50 :max-value 100}]
          result (first (fx/process (fx/make-ripple 0.1 0 1.0) pixel {:time 0}))]
      (is (= 100 (:x result)))
      (is (= 100 (:y result)))))

  (testing "preserves non-coordinate keys"
    (let [pixel  [{:x 50 :y 50 :value 42 :max-value 100}]
          result (first (fx/process (fx/make-ripple 0.1 5 1.0) pixel {:time 0}))]
      (is (= 42 (:value result)))
      (is (= 100 (:max-value result))))))

;; --- Echo ---

(deftest echo-test
  (testing "output count = input * (1 + layers)"
    (let [result (fx/process (fx/make-echo 2 10 10 2.0) plain-pixels sample-params)]
      (is (= (* 3 (count plain-pixels)) (count result)))))

  (testing "echo layers have :alpha set"
    (let [single [{:x 10 :y 20 :value 50 :max-value 100}]
          result (fx/process (fx/make-echo 3 5 5 1.0) single sample-params)
          echoes (drop 1 result)]
      (is (every? :alpha echoes))))

  (testing "original pixels appear first (no :alpha)"
    (let [single [{:x 10 :y 20 :value 50 :max-value 100}]
          result (fx/process (fx/make-echo 2 5 5 1.0) single sample-params)]
      (is (nil? (:alpha (first result)))))))

;; --- HueShift ---

(deftest hue-shift-test
  (testing "shifts r/g/b values by the given amount (mod 255)"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100 :r 100 :g 150 :b 200}
          result (first (fx/process (fx/make-hue-shift 50) [pixel] sample-params))]
      (is (= (mod (+ 100 50) 255) (:r result)))
      (is (= (mod (+ 150 50) 255) (:g result)))
      (is (= (mod (+ 200 50) 255) (:b result)))))

  (testing "wraps around 255"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100 :r 250 :g 250 :b 250}
          result (first (fx/process (fx/make-hue-shift 10) [pixel] sample-params))]
      (is (= (mod 260 255) (:r result)))))

  (testing "passes through pixels without r/g/b unchanged"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100}
          result (first (fx/process (fx/make-hue-shift 50) [pixel] sample-params))]
      (is (= pixel result)))))

;; --- BrightnessContrast ---

(deftest brightness-contrast-test
  (testing "applies brightness offset and contrast scale"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100 :r 100 :g 100 :b 100}
          result (first (fx/process (fx/make-brightness-contrast 10 1.5) [pixel] sample-params))
          expected (-> 100 (+ 10) (* 1.5) (max 0) (min 255))]
      (is (= expected (:r result)))
      (is (= expected (:g result)))
      (is (= expected (:b result)))))

  (testing "clamps output to 255 on overflow"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100 :r 200 :g 200 :b 200}
          result (first (fx/process (fx/make-brightness-contrast 0 100) [pixel] sample-params))]
      (is (= 255 (:r result)))))

  (testing "clamps output to 0 on underflow"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100 :r 5 :g 5 :b 5}
          result (first (fx/process (fx/make-brightness-contrast -100 1.0) [pixel] sample-params))]
      (is (= 0 (:r result)))))

  (testing "passes through pixels without r/g/b unchanged"
    (let [pixel  {:x 0 :y 0 :value 50 :max-value 100}
          result (first (fx/process (fx/make-brightness-contrast 10 1.5) [pixel] sample-params))]
      (is (= pixel result)))))
