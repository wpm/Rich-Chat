## Math

Inline math sits in a sentence, like $e^{i\pi} + 1 = 0$ or $\binom{n}{k} = \frac{n!}{k!\,(n-k)!}$, and stays in the line with the words around it, at the size of the text. Display math gets its own line, set in Latin Modern, the TeX font, as native MathML with no JavaScript, so it scales and prints as sharply as the text around it:

$$
\oint_{\partial\Omega} \mathbf{F} \cdot d\mathbf{S}
  = \iiint_{\Omega} \left( \nabla \cdot \mathbf{F} \right) dV
$$

Maxwell's equations, aligned:

$$
\begin{aligned}
  \nabla \cdot \mathbf{E} &= \frac{\rho}{\varepsilon_0} &
  \nabla \times \mathbf{E} &= -\frac{\partial \mathbf{B}}{\partial t} \\[4pt]
  \nabla \cdot \mathbf{B} &= 0 &
  \nabla \times \mathbf{B} &= \mu_0 \mathbf{J} + \mu_0 \varepsilon_0 \frac{\partial \mathbf{E}}{\partial t}
\end{aligned}
$$

And a piecewise definition with a matrix:

$$
f(x) =
\begin{cases}
  \displaystyle \sum_{n=0}^{\infty} \frac{(-1)^n}{(2n+1)!}\, x^{2n+1} & \text{if } |x| < \pi \\[6pt]
  \det \begin{pmatrix} \cos x & -\sin x \\ \sin x & \cos x \end{pmatrix} & \text{otherwise}
\end{cases}
$$
