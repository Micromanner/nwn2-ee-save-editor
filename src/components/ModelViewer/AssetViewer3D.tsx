import { useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { DropdownSelect } from '@/components/shared/DropdownSelect';
import { useModelLoader } from './useModelLoader';
import { useThreeScene, clearSceneModels, frameBounds } from './useThreeScene';
import { LoadingState } from '@/components/shared';

interface AssetViewer3DProps {
  resref: string;
}

export function AssetViewer3D({ resref }: AssetViewer3DProps) {
  const t = useTranslations();
  const { loadModel, loading, error } = useModelLoader();
  const { container: containerRef, scene: sceneRef, camera: cameraRef, controls: controlsRef, shadingMode, setShadingMode } = useThreeScene();

  useEffect(() => {
    if (!resref || !sceneRef.current || !cameraRef.current || !controlsRef.current) return;

    const scene = sceneRef.current;
    clearSceneModels(scene);

    loadModel(resref, scene).then((result) => {
      if (result && cameraRef.current && controlsRef.current) {
        const modelGroup = scene.getObjectByName('__model');
        if (modelGroup) {
          frameBounds(cameraRef.current, controlsRef.current, scene, modelGroup);
        }
      }
    });
  }, [resref, loadModel, sceneRef, cameraRef, controlsRef]);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', minHeight: 400 }}>
      <div ref={containerRef} style={{ width: '100%', height: '100%' }} />
      <div style={{ position: 'absolute', top: 8, right: 8, zIndex: 10 }}>
        <DropdownSelect
          small
          label={t('modelViewer.shading')}
          value={t(shadingMode === 'game' ? 'modelViewer.gameStyle' : 'modelViewer.physical')}
          items={[
            { id: 0, label: t('modelViewer.physical') },
            { id: 1, label: t('modelViewer.gameStyle') },
          ]}
          onChange={(id) => setShadingMode(id === 1 ? 'game' : 'physical')}
        />
      </div>
      
      {loading && (
        <LoadingState variant="overlay" text={t('modelViewer.loading')} />
      )}
      {error && (
        <div style={{ position: 'absolute', top: 8, left: 8, color: '#ff4444' }}>
          {error}
        </div>
      )}
    </div>
  );
}
