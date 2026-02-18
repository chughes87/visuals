(ns patch-test
  (:require [clojure.test :refer [deftest is testing]]
            [patch      :as patch]
            [generators :as gen]
            [effects    :as fx]
            [modulators :as mod]))

(def test-generator (gen/make-mandelbrot))
(def test-effect    (fx/make-color-mapper :classic))
(def test-params    {:width 10 :height 10 :center-x 0 :center-y 0
                     :zoom 1.0 :max-iter 10 :time 0})

;; --- create-patch ---

(deftest create-patch-test
  (let [p (patch/create-patch test-generator [test-effect] [] test-params)]
    (testing "stores generator"
      (is (= test-generator (:generator p))))
    (testing "stores effect chain"
      (is (= [test-effect] (:effects p))))
    (testing "stores empty modulators"
      (is (= [] (:modulators p))))
    (testing "stores initial params"
      (is (= test-params (:params p))))))

;; --- update-patch-params ---

(deftest update-patch-params-test
  (let [p (patch/create-patch test-generator [] [] test-params)]
    (testing "merges new values into params"
      (let [updated (patch/update-patch-params p {:zoom 3.0 :max-iter 50})]
        (is (= 3.0 (get-in updated [:params :zoom])))
        (is (= 50  (get-in updated [:params :max-iter])))))

    (testing "preserves keys not mentioned in update"
      (let [updated (patch/update-patch-params p {:zoom 2.0})]
        (is (= (:center-x test-params) (get-in updated [:params :center-x])))))

    (testing "does not mutate the original patch"
      (patch/update-patch-params p {:zoom 99.0})
      (is (= 1.0 (get-in p [:params :zoom]))))))

;; --- add-effect / remove-effect / replace-effect ---

(deftest add-effect-test
  (testing "appends to an empty chain"
    (let [p       (patch/create-patch test-generator [] [] test-params)
          updated (patch/add-effect p test-effect)]
      (is (= [test-effect] (:effects updated)))))

  (testing "appends after existing effects"
    (let [e2      (fx/make-hue-shift 30)
          p       (patch/create-patch test-generator [test-effect] [] test-params)
          updated (patch/add-effect p e2)]
      (is (= [test-effect e2] (:effects updated))))))

(deftest remove-effect-test
  (let [e2 (fx/make-hue-shift 30)
        p  (patch/create-patch test-generator [test-effect e2] [] test-params)]
    (testing "removes the effect at the given index"
      (is (= [e2] (:effects (patch/remove-effect p 0)))))
    (testing "removing last index leaves only first"
      (is (= [test-effect] (:effects (patch/remove-effect p 1)))))))

(deftest replace-effect-test
  (let [e2 (fx/make-hue-shift 30)
        p  (patch/create-patch test-generator [test-effect] [] test-params)]
    (testing "replaces effect at the given index"
      (let [updated (patch/replace-effect p 0 e2)]
        (is (= [e2] (:effects updated)))))))

;; --- add-modulator / replace-generator ---

(deftest add-modulator-test
  (let [m (mod/make-lfo 0.5 :sine)
        p (patch/create-patch test-generator [] [] test-params)]
    (testing "appends modulator"
      (is (= [m] (:modulators (patch/add-modulator p m)))))))

(deftest replace-generator-test
  (let [new-gen (gen/make-julia -0.7 0.27)
        p       (patch/create-patch test-generator [] [] test-params)]
    (testing "swaps the generator"
      (is (= new-gen (:generator (patch/replace-generator p new-gen)))))))

;; --- process-patch ---

(deftest process-patch-test
  (testing "returns a map with :pixel-data and :params"
    (let [result (patch/process-patch
                   (patch/create-patch test-generator [test-effect] [] test-params))]
      (is (contains? result :pixel-data))
      (is (contains? result :params))
      (is (seq (:pixel-data result)))))

  (testing "ModMatrix modulator updates params before generation"
    (let [fixed-mod (reify mod/Modulator (modulate [_ _] 1.0))
          matrix    (mod/make-mod-matrix [{:modulator fixed-mod
                                           :param :zoom
                                           :min 1.0 :max 5.0}])
          p         (patch/create-patch test-generator [] [matrix] test-params)
          result    (patch/process-patch p)]
      (is (= 5.0 (get-in result [:params :zoom])))))

  (testing "non-ModMatrix modulators in top-level list are skipped"
    (let [lfo    (mod/make-lfo 1.0 :sine)
          p      (patch/create-patch test-generator [] [lfo] test-params)
          result (patch/process-patch p)]
      ;; zoom should be unchanged since LFO is not a ModMatrix
      (is (= (:zoom test-params) (get-in result [:params :zoom])))))

  (testing "effects are applied in order"
    (let [hue-shifted-mapper (fx/make-hue-shift 100)
          p      (patch/create-patch test-generator [test-effect hue-shifted-mapper] [] test-params)
          result (patch/process-patch p)]
      ;; Just verifying the pipeline runs without error and produces output
      (is (seq (:pixel-data result)))))

  (testing "result contains an updated :patch"
    (let [p      (patch/create-patch test-generator [test-effect] [] test-params)
          result (patch/process-patch p)]
      (is (contains? result :patch))))

  (testing "cache is populated after first process-patch"
    (let [p      (patch/create-patch test-generator [] [] test-params)
          result (patch/process-patch p)]
      (is (some? (get-in result [:patch :gen-cache])))
      (is (some? (get-in result [:patch :last-gen-params])))))

  (testing "cache is reused when generator-relevant params are unchanged"
    (let [p       (patch/create-patch test-generator [] [] test-params)
          result1 (patch/process-patch p)
          cached  (get-in result1 [:patch :gen-cache])
          ;; Second call with same params - only :time changes, which
          ;; MandelbrotGenerator does not read
          p2      (patch/update-patch-params (:patch result1) {:time 99.0})
          result2 (patch/process-patch p2)]
      (is (identical? cached (get-in result2 [:patch :gen-cache])))))

  (testing "cache is invalidated when generator-relevant params change"
    (let [p       (patch/create-patch test-generator [] [] test-params)
          result1 (patch/process-patch p)
          cached  (get-in result1 [:patch :gen-cache])
          ;; Change max-iter, which MandelbrotGenerator reads
          p2      (patch/update-patch-params (:patch result1) {:max-iter 5})
          result2 (patch/process-patch p2)]
      (is (not (identical? cached (get-in result2 [:patch :gen-cache])))))))
