"""The statistics the scoring needs, in the standard library so the public kit runs anywhere."""
import math
import random
import statistics


def log_error(forecast, final):
    """Gap between forecast and final price in points: 100 x |ln(forecast / final)|.

    Close to the percentage gap when small, and a 2x miss is 69.3 points in either direction.
    """
    return 100.0 * abs(math.log(forecast / final))


def _betacf(a, b, x):
    tiny = 1e-300
    c, d = 1.0, 1.0 - (a + b) * x / (a + 1.0)
    d = 1.0 / (d if abs(d) > tiny else tiny)
    h = d
    for m in range(1, 400):
        for num in (m * (b - m) * x / ((a + 2 * m - 1) * (a + 2 * m)),
                    -(a + m) * (a + b + m) * x / ((a + 2 * m) * (a + 2 * m + 1))):
            d = 1.0 + num * d
            d = 1.0 / (d if abs(d) > tiny else tiny)
            c = 1.0 + num / c
            c = c if abs(c) > tiny else tiny
            delta = d * c
            h *= delta
        if abs(delta - 1.0) < 1e-14:
            break
    return h


def betainc(a, b, x):
    """Regularised incomplete beta function."""
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    front = math.exp(math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b)
                     + a * math.log(x) + b * math.log(1.0 - x))
    if x < (a + 1.0) / (a + b + 2.0):
        return front * _betacf(a, b, x) / a
    return 1.0 - front * _betacf(b, a, 1.0 - x) / b


def t_cdf(t, df):
    tail = 0.5 * betainc(df / 2.0, 0.5, df / (df + t * t))
    return 1.0 - tail if t > 0 else tail


def t_quantile(p, df):
    low, high = -1e3, 1e3
    for _ in range(200):
        mid = (low + high) / 2.0
        low, high = (mid, high) if t_cdf(mid, df) < p else (low, mid)
    return (low + high) / 2.0


def paired(diffs, margin):
    """Paired t-test on per-auction differences, and two one-sided tests for equivalence.

    `equivalent_p` is the larger of the two one-sided p-values: the differences count as within
    +/- margin only when it is below 0.05.
    """
    n = len(diffs)
    if n < 2:
        return {"n": n}
    mean, sd = statistics.fmean(diffs), statistics.stdev(diffs)
    se, df = sd / math.sqrt(n), n - 1
    if se == 0:
        return {"n": n, "mean": mean, "sd": 0.0}
    half = t_quantile(0.975, df) * se
    return {
        "n": n, "mean": mean, "sd": sd, "se": se,
        "ci95": [mean - half, mean + half],
        "t": mean / se,
        "difference_p": 2.0 * (1.0 - t_cdf(abs(mean / se), df)),
        "equivalent_p": max(1.0 - t_cdf((mean + margin) / se, df), t_cdf((mean - margin) / se, df)),
    }


def bootstrap_mean(values, seed, draws=2000):
    """Mean with a 95% range from resampling auctions."""
    rng = random.Random(seed)
    n = len(values)
    means = sorted(statistics.fmean(rng.choices(values, k=n)) for _ in range(draws))
    return statistics.fmean(values), [means[int(0.025 * draws)], means[int(0.975 * draws) - 1]]


def auctions_needed(sd_diff, gap, power_z=0.8416, alpha_z=1.96):
    """Auctions for 80% power to see a paired gap of `gap` points at the 5% level (normal approximation)."""
    return math.ceil(((alpha_z + power_z) * sd_diff / gap) ** 2)
