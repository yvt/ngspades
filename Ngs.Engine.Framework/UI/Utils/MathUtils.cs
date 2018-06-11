//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI.Utils {
    static class MathUtils {
        /// <summary>
        /// Returns the approximate value of the gaussian error function at <paramref name="x"/>.
        /// </summary>
        public static double Erf(double x) {
            const double a1 = 0.254829592;
            const double a2 = -0.284496736;
            const double a3 = 1.421413741;
            const double a4 = -1.453152027;
            const double a5 = 1.061405429;
            const double p = 0.3275911;

            double t = 1.0 / (1.0 + p * Math.Abs(x));
            double y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.Exp(-x * x);

            return (x < 0.0 ? -y : y);
        }
    }
}