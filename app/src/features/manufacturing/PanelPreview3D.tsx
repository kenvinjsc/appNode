// Small isolated 3D preview of one panel (reuses meshes already on the GPU).
import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { ObjectId } from '../../core-api/types';
import { getEngine, useSceneRevision } from '../../viewport/viewportBus';
import { surfaceMaterial, isGrained, edgeMaterial } from '../../viewport/materials/materials';

export function PanelPreview3D({ id }: { id: ObjectId }) {
  const host = useRef<HTMLDivElement>(null);
  const rev = useSceneRevision((s) => s.rev);
  useEffect(() => {
    const el = host.current!;
    const engine = getEngine();
    const entry = engine?.entries.get(id);
    const geo = entry && engine!.geometries.get(entry.ro.geometry_key);
    if (!entry || !geo) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    el.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color('#f1f3f5');
    scene.add(new THREE.HemisphereLight('#fff', '#999', 1.2));
    const d = new THREE.DirectionalLight('#fff', 1.4);
    d.position.set(1, 2, 3);
    scene.add(d);
    const mesh = new THREE.Mesh(geo.surface, surfaceMaterial(entry.ro.color, 'normal', isGrained(entry.ro.material_id)));
    const edges = new THREE.LineSegments(geo.edges, edgeMaterial('normal'));
    const box = geo.surface.boundingBox!.clone();
    const c = box.getCenter(new THREE.Vector3());
    const g = new THREE.Group();
    g.add(mesh, edges);
    g.position.sub(c);
    scene.add(g);
    const r = box.getSize(new THREE.Vector3()).length() / 2;
    const cam = new THREE.PerspectiveCamera(35, 1, 1, r * 20);
    const dist = r / Math.sin(THREE.MathUtils.degToRad(17.5));
    cam.position.set(0.45, 0.3, 1).normalize().multiplyScalar(dist);
    const controls = new OrbitControls(cam, renderer.domElement);
    let raf = 0;
    const resize = () => {
      const w = el.clientWidth || 1;
      const h = el.clientHeight || 1;
      renderer.setSize(w, h);
      cam.aspect = w / h;
      cam.updateProjectionMatrix();
    };
    const ro = new ResizeObserver(resize);
    ro.observe(el);
    resize();
    const loop = () => {
      raf = requestAnimationFrame(loop);
      controls.update();
      renderer.render(scene, cam);
    };
    loop();
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      controls.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [id, rev]);
  return <div className="preview3d" ref={host} />;
}
