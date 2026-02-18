(ns generators-test
  (:require [clojure.test :refer [deftest is testing]]
            [generators :as gen]))

;; --- Pure math functions ---

(deftest mandelbrot-iterations-test
  (testing "origin stays in set (returns max-iter)"
    (is (= 100 (gen/mandelbrot-iterations 0 0 100))))
  (testing "far point escapes immediately"
    (is (< (gen/mandelbrot-iterations 2 2 100) 5)))
  (testing "respects max-iter cap"
    (is (= 50 (gen/mandelbrot-iterations 0 0 50))))
  (testing "returns non-negative value"
    (is (>= (gen/mandelbrot-iterations -0.5 0.5 100) 0))))

(deftest julia-iterations-test
  (testing "far point escapes quickly"
    (is (< (gen/julia-iterations 2 2 -0.7 0.27015 100) 10)))
  (testing "result is bounded by max-iter"
    (is (<= (gen/julia-iterations 0 0 -0.7 0.27015 50) 50)))
  (testing "result is non-negative"
    (is (>= (gen/julia-iterations 0 0 -0.7 0.27015 100) 0))))

(deftest burning-ship-iterations-test
  (testing "origin stays in set (returns max-iter)"
    (is (= 100 (gen/burning-ship-iterations 0 0 100))))
  (testing "far point escapes quickly"
    (is (< (gen/burning-ship-iterations 2 2 100) 5)))
  (testing "respects max-iter cap"
    (is (= 30 (gen/burning-ship-iterations 0 0 30)))))

;; --- Generator output structure ---

(def base-params
  {:width 10 :height 10 :center-x 0 :center-y 0 :zoom 1.0 :max-iter 10})

(deftest mandelbrot-generator-test
  (let [g (gen/make-mandelbrot)
        result (gen/generate g base-params)]
    (testing "returns a non-empty sequence"
      (is (seq result)))
    (testing "every pixel has required keys"
      (is (every? #(and (contains? % :x)
                        (contains? % :y)
                        (contains? % :value)
                        (contains? % :max-value))
                  result)))
    (testing "pixel values are bounded by max-iter"
      (is (every? #(<= (:value %) (:max-iter base-params)) result)))
    (testing "pixel coordinates are within canvas bounds"
      (is (every? #(and (< (:x %) (:width base-params))
                        (< (:y %) (:height base-params)))
                  result)))))

(deftest julia-generator-test
  (let [g (gen/make-julia -0.7 0.27015)
        result (gen/generate g base-params)]
    (testing "returns a non-empty sequence"
      (is (seq result)))
    (testing "every pixel has required keys"
      (is (every? #(and (contains? % :x) (contains? % :y)
                        (contains? % :value) (contains? % :max-value))
                  result)))))

(deftest burning-ship-generator-test
  (let [g (gen/make-burning-ship)
        result (gen/generate g base-params)]
    (testing "returns a non-empty sequence"
      (is (seq result)))
    (testing "every pixel has required keys"
      (is (every? #(and (contains? % :x) (contains? % :y)
                        (contains? % :value) (contains? % :max-value))
                  result)))))

;; --- gen-params ---

(deftest gen-params-test
  (testing "mandelbrot declares fractal keys"
    (is (= #{:width :height :center-x :center-y :zoom :max-iter}
           (gen/gen-params (gen/make-mandelbrot)))))
  (testing "julia declares fractal keys"
    (is (= #{:width :height :center-x :center-y :zoom :max-iter}
           (gen/gen-params (gen/make-julia -0.7 0.27015)))))
  (testing "burning-ship declares fractal keys"
    (is (= #{:width :height :center-x :center-y :zoom :max-iter}
           (gen/gen-params (gen/make-burning-ship)))))
  (testing "noise declares time-dependent keys"
    (is (= #{:width :height :time}
           (gen/gen-params (gen/make-noise 0.01 4)))))
  (testing "noise gen-params includes :time, unlike fractal generators"
    (is (contains? (gen/gen-params (gen/make-noise 0.01 4)) :time))
    (is (not (contains? (gen/gen-params (gen/make-mandelbrot)) :time)))))
