import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D
import math


def filter(resistance, capacitance, frequency):
    reactance = 1/(2 * math.pi * frequency * capacitance)
    impedance = math.sqrt(resistance**2 + reactance**2)
    return reactance / impedance


if __name__ == "__main__":
    capacitance = 10**-6
    resistance = 1000

    freqs = np.array([x+0.1 for x in range(0, 10**5, 10)])
    output = [filter(resistance, capacitance, f) for f in freqs]


    plt.loglog(freqs, output)
    plt.show()
    """
    capacitance = 10**-6
    resistance = 100

    resistances = np.array([x for x in range(0, 100, 1)])
    capacitances = np.linspace(10**-3, 10**-9, 1000);
    freqs = np.array([x+0.1 for x in range(0, 10**6, 1000)])
    (resistances, freqs) = np.meshgrid(capacitances, freqs)
    # output = [filter(capacitance, resistance, f) for f in freqs]
    filter_v = np.vectorize(lambda c, f: filter(resistance, c, f))
    output = filter_v(freqs, capacitances)


    fig = plt.figure();
    ax = fig.gca(projection = '3d')
    # plt.loglog(freqs, output)
    ax.plot_surface(capacitances, freqs, output)
    plt.show()
    """
