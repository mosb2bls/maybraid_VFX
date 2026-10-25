"""Restrained, engine-independent organization reference; metres/RH/+Y.

Reuse local_deformation and its unchanged clips. Supply original GLB POSITION
arrays to parent_state/instances for exact support on the rendered triangles.
No Houdini, random stream, per-frame integration, or recursive spawning.
"""
from functools import lru_cache
from pathlib import Path
import numpy as np
import local_deformation as base

A = Path(__file__).resolve().parents[1]
VARIANTS = ('a_shoulder', 'b_stack', 'c_swept')
# parent: (age onset, duration, angle degrees, forward metres, outward metres)
LATE = {0: (1.50, 1.65, 7., .13, -.065),
        3: (1.72, 1.55, 8., .15, .075),
        5: (1.88, 1.45, 6., .11, -.045),
        9: (1.58, 1.80, 9., .16, -.075)}
# parent, global birth, rest shoulder anchor, final size relative to parent,
# outward/up displacement (world Y, parent's horizontal frame), independent clip
CHILDREN = (
    dict(parent=3, birth=1.22, anchor=[.25, .64, .18], ratio=.34,
         extra=[.16, .07, .12], clip=1, phase_offset=.13, phase_speed=.88),
    dict(parent=0, birth=2.08, anchor=[.18, .76, -.25], ratio=.30,
         extra=[.21, .17, -.24], clip=2, phase_offset=.29, phase_speed=.94),
    dict(parent=9, birth=2.71, anchor=[.28, .78, -.20], ratio=.32,
         extra=[.22, .16, -.20], clip=0, phase_offset=.07, phase_speed=.81),
)

@lru_cache(None)
def rest_points(variant):
    from portable_io import read_glb
    names=('Shoulder','Column','Swept')
    return read_glb(A/'Assets/Body'/(names[variant]+'.glb'))[0]

def yaw_matrix(y):
    return np.array([[np.cos(y), 0, -np.sin(y)], [0, 1, 0],
                     [np.sin(y), 0, np.cos(y)]])

def axis_matrix(axis, angle):
    axis = np.asarray(axis, dtype=float); axis /= np.linalg.norm(axis)
    x, y, z = axis
    K = np.array([[0, -z, y], [z, 0, -x], [-y, x, 0]])
    return np.eye(3) + np.sin(angle)*K + (1-np.cos(angle))*(K@K)

def world(p, state):
    q = base.forward(p, state['coefficients'])*state['scale']
    return (q-state['pivot'])@state['rotation'].T + state['pivot'] + state['position']

def rest_from_world(w, state):
    q = (np.asarray(w)-state['position']-state['pivot'])@state['rotation'] + state['pivot']
    return base.inverse(q/state['scale'], state['coefficients'])

def world_normals(p, n, state):
    out = (base.normals(p, n, state['coefficients'])/state['scale'])@state['rotation'].T
    return out/np.linalg.norm(out, axis=-1, keepdims=True)

def parent_state(t, i, late=True, points=None):
    old = base.pose(float(t), i)
    if old is None:
        return None
    s = dict(old, id=i, parent=None, age=t-old['birth'],
             coefficients=base.controls(old['phase'], old['clip']),
             rotation=yaw_matrix(old['yaw']), pivot=np.zeros(3),
             late_progress=0., tilt_degrees=0., support_correction_m=0.)
    if not late or i not in LATE:
        return s
    start, duration, degrees, dx, dz = LATE[i]
    u = float(base.S((s['age']-start)/duration))
    if u == 0:
        return s
    P = rest_points(s['variant']) if points is None else points
    baseline_floor = float(world(P, s)[:, 1].min())
    # A low local pivot and a rolling axis tangent to the outward path.
    s['pivot'] = np.array([.10, .12, 0.])*s['scale']
    s['rotation'] = yaw_matrix(s['yaw'])@axis_matrix([np.sign(dz)*.24, 0, -1], np.deg2rad(degrees)*u)
    s['position'] = s['position'] + yaw_matrix(s['yaw'])@np.array([dx*u, .035*u*u, dz*u*u])
    # Preserve the unchanged baseline's minimum Y, including its existing arch.
    # The support lift is part of the WORLD transform shared with interior data.
    correction = baseline_floor - float(world(P, s)[:, 1].min())
    s['position'][1] += correction
    s.update(late_progress=u, tilt_degrees=degrees*u, support_correction_m=correction)
    return s

@lru_cache(None)
def child_initial(k, late=True):
    rule = CHILDREN[k]; t = rule['birth']; i = rule['parent']
    parent = parent_state(t, i, late)
    anchor = np.array(rule['anchor'])
    # Velocity of a material point, including local deformation, root travel,
    # tilt, scale and support; use the same rest anchor at t +/- 1 ms.
    dt = .001
    velocity = (world(anchor, parent_state(t+dt, i, late))-
                world(anchor, parent_state(t-dt, i, late)))/(2*dt)
    return world(anchor, parent), velocity, parent['scale'].copy(), parent['yaw'], parent['variant']

def child_state(t, k, late=True):
    rule = CHILDREN[k]; age = t-rule['birth']
    if age < 0:
        return None
    anchor, velocity, parent_scale, yaw, variant = child_initial(k, late)
    # Size starts at 4% of the parent's axis scale, concealed in its interior.
    size = .04 + (rule['ratio']-.04)*base.S(age/.68)
    scale = parent_scale*size
    phase = float(np.clip(rule['phase_offset']+age*rule['phase_speed'], 0, 3))
    c = base.controls(phase, rule['clip'])
    R = yaw_matrix(yaw)
    # Anchor the child at a rest material point rather than at its bottom.
    center = base.forward(np.array([.05, .66, 0.]), c)*scale
    # Initial velocity is exactly 65% of the material-point velocity. Its
    # contribution decays once (0.85s); no per-frame Euler integration.
    motion = .65*velocity*.85*(1-np.exp(-age/.85))
    extra = R@np.array(rule['extra'])*base.S(age/1.12)
    current_anchor = world(np.array(rule['anchor']), parent_state(t, rule['parent'], late))
    attachment = .85*base.S(age/.72)
    shoulder = anchor+motion+attachment*(current_anchor-anchor-motion)
    position = shoulder + extra - R@center
    return dict(id=12+k, parent=rule['parent'], birth=rule['birth'], age=age,
                position=position, rotation=R, pivot=np.zeros(3), scale=scale,
                yaw=yaw, phase=phase, clip=rule['clip'], variant=variant,
                coefficients=c, inherited_material_velocity=velocity,
                late_progress=0., tilt_degrees=0., support_correction_m=0.)

def instances(t, late=True, children=True):
    states = [parent_state(t, i, late) for i in range(12)]
    if children:
        states += [child_state(t, k, late) for k in range(len(CHILDREN))]
    return [s for s in states if s is not None]

def conservative_world_bounds(state):
    # The body contract's domain bound encloses all deformed density.
    lo = np.array([-2.042, -.444, -1.817])
    hi = np.array([1.917, 3.531, 1.786])
    corners = np.array([[x,y,z] for x in [lo[0],hi[0]]
                        for y in [lo[1],hi[1]] for z in [lo[2],hi[2]]])
    q = (corners*state['scale']-state['pivot'])@state['rotation'].T+state['pivot']+state['position']
    return q.min(0), q.max(0)
