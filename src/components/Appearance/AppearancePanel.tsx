import { Fragment, useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Button, Card, Elevation, InputGroup, Menu, MenuItem, NonIdealState, Popover, Slider, Spinner, Switch } from '@blueprintjs/core';
import { GiMirrorMirror, GiMagnifyingGlass } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { CharacterStateAPI } from '@/lib/api/character-state';
import { T } from '../theme';
import { KVRow } from '../shared';
import type { AppearanceOption, AppearanceUpdates, AvailableRace, AvailableSubrace, PartTintCapability, TintChannel, TintChannels, VoiceSetInfo } from '@/lib/bindings';
import { invoke } from '@tauri-apps/api/core';
import { CharacterViewer3D } from './CharacterViewer3D';
import { VariantStepper } from './VariantStepper';
import { ColorPicker } from './ColorPicker';

type PartType = 'head' | 'hair' | 'fhair' | 'wings' | 'tail' | 'helm' | 'body';

function SectionHeader({ label }: { label: string }) {
  return (
    <div className="t-section" style={{ color: T.accent, marginBottom: 8, marginTop: 4 }}>
      {label}
    </div>
  );
}

export function AppearancePanel() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { character } = useCharacterContext();
  const appearanceSubsystem = useSubsystem('appearance');

  const [wingOptions, setWingOptions] = useState<AppearanceOption[]>([]);
  const [tailOptions, setTailOptions] = useState<AppearanceOption[]>([]);
  const [raceOptions, setRaceOptions] = useState<AvailableRace[]>([]);
  const [subraceOptions, setSubraceOptions] = useState<AvailableSubrace[]>([]);
  const [modelRefreshKey, setModelRefreshKey] = useState(0);
  const [partRefresh, setPartRefresh] = useState<{ parts: PartType[]; key: number } | null>(null);
  const [liveHeight, setLiveHeight] = useState(0.95);
  const [liveGirth, setLiveGirth] = useState(0.95);
  const [voicesets, setVoicesets] = useState<VoiceSetInfo[]>([]);
  const [voiceFilter, setVoiceFilter] = useState('');
  const [playingResref, setPlayingResref] = useState<string | null>(null);
  const [pendingVoiceId, setPendingVoiceId] = useState<number | null>(null);
  const [pendingTints, setPendingTints] = useState<{ tint_head?: TintChannels; tint_hair?: TintChannels } | null>(null);
  const [pendingBodyTint, setPendingBodyTint] = useState<TintChannels | null>(null);
  const [pendingSize, setPendingSize] = useState<{ height: number; girth: number } | null>(null);
  const [showHelmetInViewer, setShowHelmetInViewer] = useState(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const blobUrlRef = useRef<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (character?.id) {
      if (!appearanceSubsystem.data && !appearanceSubsystem.isLoading) {
        appearanceSubsystem.load();
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id]);

  const dataNeverDrawHelmet = appearanceSubsystem.data?.never_draw_helmet;
  useEffect(() => {
    console.log("AppearancePanel mount/update. dataNeverDrawHelmet:", dataNeverDrawHelmet);
    if (dataNeverDrawHelmet !== undefined) {
      console.log("Setting showHelmetInViewer to", !dataNeverDrawHelmet);
      setShowHelmetInViewer(!dataNeverDrawHelmet);
    }
  }, [dataNeverDrawHelmet]);

  useEffect(() => {
    async function loadOptions() {
      try {
        const [wings, tails, voices, races, subraces] = await Promise.all([
          CharacterStateAPI.getAvailableWings(),
          CharacterStateAPI.getAvailableTails(),
          CharacterStateAPI.getAvailableVoicesets(),
          invoke<AvailableRace[]>('get_available_races'),
          invoke<AvailableSubrace[]>('get_all_playable_subraces'),
        ]);
        setWingOptions(wings);
        setTailOptions(tails);
        setVoicesets(voices);
        setRaceOptions(races);
        setSubraceOptions(subraces);
      } catch (err) {
        handleError(err);
      }
    }
    if (character?.id) {
      loadOptions();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id]);

  const updateField = useCallback(async (updates: AppearanceUpdates, refreshParts?: PartType | PartType[]) => {
    try {
      const result = await CharacterStateAPI.updateAppearance(updates);
      appearanceSubsystem.updateData(result);
      if (refreshParts) {
        const parts = Array.isArray(refreshParts) ? refreshParts : [refreshParts];
        if (debounceRef.current) clearTimeout(debounceRef.current);
        debounceRef.current = setTimeout(() => {
          setPartRefresh({ parts, key: Date.now() });
        }, 300);
      }
    } catch (err) {
      handleError(err);
    }
  }, [appearanceSubsystem, handleError]);

  const updateTintChannel = useCallback(
    (group: 'tint_head' | 'tint_hair', channelKey: 'channel1' | 'channel2' | 'channel3') =>
      (value: TintChannel) => {
        const data = appearanceSubsystem.data;
        if (!data) return;
        setPendingTints(prev => {
          const base = prev ?? {};
          const groupValue = base[group] ?? { ...data[group] };
          return { ...base, [group]: { ...groupValue, [channelKey]: value } };
        });
      },
    [appearanceSubsystem.data]
  );

  const confirmTints = useCallback(() => {
    if (!pendingTints) return;
    const updates: AppearanceUpdates = {};
    if (pendingTints.tint_head) updates.tint_head = pendingTints.tint_head;
    if (pendingTints.tint_hair) updates.tint_hair = pendingTints.tint_hair;
    updateField(updates);
    setPendingTints(null);
  }, [pendingTints, updateField]);

  const cancelTints = useCallback(() => {
    setPendingTints(null);
  }, []);

  // Tail/wing body tint has its own pending + apply, separate from the head/hair
  // Colors card, so its Apply/Cancel sits next to the pickers that armed it.
  const updateBodyTintChannel = useCallback(
    (channelKey: 'channel1' | 'channel2' | 'channel3') =>
      (value: TintChannel) => {
        const data = appearanceSubsystem.data;
        if (!data) return;
        setPendingBodyTint(prev => {
          const base = prev ?? { ...data.tint_body };
          return { ...base, [channelKey]: value };
        });
      },
    [appearanceSubsystem.data]
  );

  const confirmBodyTint = useCallback(() => {
    if (!pendingBodyTint) return;
    updateField({ tint_body: pendingBodyTint });
    setPendingBodyTint(null);
  }, [pendingBodyTint, updateField]);

  const cancelBodyTint = useCallback(() => {
    setPendingBodyTint(null);
  }, []);

  const confirmSize = useCallback(() => {
    if (!pendingSize) return;
    updateField({ height: pendingSize.height, girth: pendingSize.girth });
    setPendingSize(null);
  }, [pendingSize, updateField]);

  const cancelSize = useCallback(() => {
    if (!appearanceSubsystem.data) return;
    setLiveHeight(appearanceSubsystem.data.height);
    setLiveGirth(appearanceSubsystem.data.girth);
    setPendingSize(null);
  }, [appearanceSubsystem.data]);

  const cleanupAudio = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current = null;
    }
    if (blobUrlRef.current) {
      URL.revokeObjectURL(blobUrlRef.current);
      blobUrlRef.current = null;
    }
  }, []);

  const playVoicePreview = useCallback(async (resref: string) => {
    try {
      cleanupAudio();
      setPlayingResref(resref);

      const audioBytes = await CharacterStateAPI.previewVoiceset(resref);
      const blob = new Blob([new Uint8Array(audioBytes)], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      blobUrlRef.current = url;

      const audio = new Audio(url);
      audio.volume = 0.5;
      audioRef.current = audio;
      audio.onended = () => {
        setPlayingResref(null);
        URL.revokeObjectURL(url);
        blobUrlRef.current = null;
      };
      audio.onerror = () => {
        setPlayingResref(null);
        URL.revokeObjectURL(url);
        blobUrlRef.current = null;
      };
      await audio.play();
    } catch {
      setPlayingResref(null);
    }
  }, [cleanupAudio]);

  useEffect(() => {
    return () => { cleanupAudio(); };
  }, [cleanupAudio]);

  const raceGroups = useMemo(() => {
    type Group = { baseRaceId: number; groupName: string; race: AvailableRace | null; subraces: AvailableSubrace[] };
    const byId = new Map<number, Group>();
    for (const r of raceOptions) {
      byId.set(r.id, { baseRaceId: r.id, groupName: r.name, race: r, subraces: [] });
    }
    for (const s of subraceOptions) {
      let g = byId.get(s.base_race);
      if (!g) {
        g = { baseRaceId: s.base_race, groupName: s.base_race_name || `Race ${s.base_race}`, race: null, subraces: [] };
        byId.set(s.base_race, g);
      }
      g.subraces.push(s);
    }
    for (const g of byId.values()) {
      g.subraces.sort((a, b) => a.name.localeCompare(b.name));
      if (g.race && g.subraces.length > 0) {
        g.race = null;
      }
    }
    return Array.from(byId.values()).sort((a, b) => {
      if (!!a.race !== !!b.race) return a.race ? -1 : 1;
      return a.baseRaceId - b.baseRaceId;
    });
  }, [raceOptions, subraceOptions]);

  const currentAppearanceType = appearanceSubsystem.data?.appearance_type;
  const [pickedRaceKey, setPickedRaceKey] = useState<string | null>(null);

  const activeRaceKey = useMemo(() => {
    if (currentAppearanceType === undefined) return null;
    if (pickedRaceKey) {
      const [kind, idStr] = pickedRaceKey.split(':');
      const id = Number(idStr);
      if (kind === 'race') {
        const r = raceOptions.find(x => x.id === id);
        if (r && r.appearance === currentAppearanceType) return pickedRaceKey;
      } else if (kind === 'sub') {
        const s = subraceOptions.find(x => x.id === id);
        if (s && s.appearance === currentAppearanceType) return pickedRaceKey;
      }
    }
    for (const g of raceGroups) {
      if (g.race && g.race.appearance === currentAppearanceType) return `race:${g.race.id}`;
      const s = g.subraces.find(sr => sr.appearance === currentAppearanceType);
      if (s) return `sub:${s.id}`;
    }
    return null;
  }, [raceGroups, raceOptions, subraceOptions, currentAppearanceType, pickedRaceKey]);

  const currentRaceLabel = useMemo(() => {
    if (!activeRaceKey) return currentAppearanceType !== undefined ? `#${currentAppearanceType}` : null;
    const [kind, idStr] = activeRaceKey.split(':');
    const id = Number(idStr);
    if (kind === 'race') return raceOptions.find(r => r.id === id)?.name ?? null;
    return subraceOptions.find(s => s.id === id)?.name ?? null;
  }, [activeRaceKey, raceOptions, subraceOptions, currentAppearanceType]);

  const applyAppearanceType = useCallback(async (appearanceId: number, key: string) => {
    try {
      setPickedRaceKey(key);
      const result = await CharacterStateAPI.updateAppearance({ appearance_type: appearanceId });
      appearanceSubsystem.updateData(result);
      setModelRefreshKey(k => k + 1);
    } catch (err) {
      handleError(err);
    }
  }, [appearanceSubsystem, handleError]);

  const groupedVoicesets = useMemo(() => {
    const filtered = voicesets.filter(v =>
      voiceFilter === '' || v.name.toLowerCase().includes(voiceFilter.toLowerCase())
    );
    const grouped = filtered.reduce((acc, v) => {
      const key = v.voice_type;
      if (!acc[key]) acc[key] = [];
      acc[key].push(v);
      return acc;
    }, {} as Record<number, VoiceSetInfo[]>);
    return Object.keys(grouped).map(Number).sort((a, b) => a - b).map(typeKey => ({
      typeKey,
      voices: grouped[typeKey],
    }));
  }, [voicesets, voiceFilter]);

  const dataHeight = appearanceSubsystem.data?.height;
  const dataGirth = appearanceSubsystem.data?.girth;
  useEffect(() => { if (dataHeight !== undefined) setLiveHeight(dataHeight); }, [dataHeight]);
  useEffect(() => { if (dataGirth !== undefined) setLiveGirth(dataGirth); }, [dataGirth]);

  if (!character) {
    return <NonIdealState icon={<GameIcon icon={GiMirrorMirror} size={40} />} title={t('character.noCharacter')} description={t('character.loadSaveFile')} />;
  }

  if (!appearanceSubsystem.data) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <Spinner />
      </div>
    );
  }

  const data = appearanceSubsystem.data;

  const caps = data.color_capabilities;
  const capProps = (cap: PartTintCapability | null | undefined, ch: 0 | 1 | 2) => {
    if (!cap) return {};
    if (!cap.has_tint_map) return { disabled: true, disabledReason: t('appearance.tintNoTintMap') };
    if (!cap.channels[ch]) return { disabled: true, disabledReason: t('appearance.tintChannelUnused') };
    return {};
  };

  return (
    <div style={{ display: 'flex', height: '100%', overflow: 'hidden' }}>
      {/* Left: Controls */}
      <div style={{ width: 350, flexShrink: 0, overflow: 'auto', padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>

        {/* Head & Hair */}
        <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
          <SectionHeader label={t('appearance.headAndHair')} />
          <KVRow
            label={t('appearance.race')}
            value={
              <Popover
                content={
                  <Menu style={{ maxHeight: 400, overflowY: 'auto' }}>
                    {raceGroups.map(group => (
                      <Fragment key={group.baseRaceId}>
                        {group.race && (
                          <MenuItem
                            text={group.race.name}
                            active={activeRaceKey === `race:${group.race.id}`}
                            onClick={() => applyAppearanceType(group.race!.appearance, `race:${group.race!.id}`)}
                          />
                        )}
                        {group.subraces.map(s => (
                          <MenuItem
                            key={s.id}
                            text={s.name}
                            active={activeRaceKey === `sub:${s.id}`}
                            onClick={() => applyAppearanceType(s.appearance, `sub:${s.id}`)}
                          />
                        ))}
                      </Fragment>
                    ))}
                  </Menu>
                }
                placement="bottom-end"
                minimal
              >
                <Button
                  minimal
                  rightIcon="caret-down"
                  text={currentRaceLabel ?? `#${data.appearance_type}`}
                  className="t-semibold"
                />
              </Popover>
            }
          />
          <KVRow
            label={t('appearance.head')}
            value={<VariantStepper value={data.appearance_head} variants={data.available_heads} onChange={(v) => updateField({ appearance_head: v }, ['head', 'fhair'])} />}
          />
          <KVRow
            label={t('appearance.hairStyle')}
            value={<VariantStepper value={data.appearance_hair} variants={data.available_hairs} onChange={(v) => updateField({ appearance_hair: v }, 'hair')} />}
          />
          <KVRow
            label={t('appearance.facialHair')}
            value={
              <Switch
                checked={data.appearance_fhair > 0}
                disabled={!data.has_fhair_meshes}
                onChange={() => updateField({ appearance_fhair: data.appearance_fhair > 0 ? 0 : 1 }, 'fhair')}
                style={{ marginBottom: 0 }}
                title={!data.has_fhair_meshes ? t('appearance.noFacialHair') : undefined}
              />
            }
          />
        </Card>

        {/* Colors */}
        <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
          <SectionHeader label={t('appearance.colors')} />
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <ColorPicker label={t('appearance.skin')} value={(pendingTints?.tint_head ?? data.tint_head).channel1} onChange={updateTintChannel('tint_head', 'channel1')} {...capProps(caps.head, 0)} />
            <ColorPicker label={t('appearance.eyebrows')} value={(pendingTints?.tint_head ?? data.tint_head).channel2} onChange={updateTintChannel('tint_head', 'channel2')} {...capProps(caps.head, 1)} />
            <ColorPicker label={t('appearance.eyes')} value={(pendingTints?.tint_head ?? data.tint_head).channel3} onChange={updateTintChannel('tint_head', 'channel3')} {...capProps(caps.head, 2)} />
            <ColorPicker label={t('appearance.hairBase')} value={(pendingTints?.tint_hair ?? data.tint_hair).channel1} onChange={updateTintChannel('tint_hair', 'channel1')} {...capProps(caps.hair, 0)} />
            <ColorPicker label={t('appearance.hairHighlight')} value={(pendingTints?.tint_hair ?? data.tint_hair).channel2} onChange={updateTintChannel('tint_hair', 'channel2')} {...capProps(caps.hair, 1)} />
            <ColorPicker label={t('appearance.hairAccessory')} value={(pendingTints?.tint_hair ?? data.tint_hair).channel3} onChange={updateTintChannel('tint_hair', 'channel3')} {...capProps(caps.hair, 2)} />
          </div>
          {pendingTints !== null && (
            <div style={{ marginTop: 8, display: 'flex', justifyContent: 'flex-end', gap: 6 }}>
              <Button
                small
                text={t('common.cancel')}
                onClick={cancelTints}
              />
              <Button
                small
                intent="primary"
                text={t('actions.apply')}
                onClick={confirmTints}
              />
            </div>
          )}
        </Card>

        {/* Size */}
        <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
          <SectionHeader label={t('appearance.size')} />
          <KVRow
            label={t('appearance.height')}
            value={
              <div style={{ display: 'flex', alignItems: 'center', gap: 12, flex: 1 }}>
                <Slider
                  min={0.95} max={1.05} stepSize={0.01}
                  value={liveHeight}
                  onChange={(v) => {
                    setLiveHeight(v);
                    setPendingSize(prev => ({ height: v, girth: prev?.girth ?? liveGirth }));
                  }}
                  labelRenderer={false}
                />
                <span className="t-md t-semibold" style={{ minWidth: 36, textAlign: 'right' }}>{liveHeight.toFixed(2)}</span>
              </div>
            }
          />
          <KVRow
            label={t('appearance.girth')}
            value={
              <div style={{ display: 'flex', alignItems: 'center', gap: 12, flex: 1 }}>
                <Slider
                  min={0.95} max={1.05} stepSize={0.01}
                  value={liveGirth}
                  onChange={(v) => {
                    setLiveGirth(v);
                    setPendingSize(prev => ({ height: prev?.height ?? liveHeight, girth: v }));
                  }}
                  labelRenderer={false}
                />
                <span className="t-md t-semibold" style={{ minWidth: 36, textAlign: 'right' }}>{liveGirth.toFixed(2)}</span>
              </div>
            }
          />
          {pendingSize !== null && (
            <div style={{ marginTop: 8, display: 'flex', justifyContent: 'flex-end', gap: 6 }}>
              <Button small text={t('common.cancel')} onClick={cancelSize} />
              <Button small intent="primary" text={t('actions.apply')} onClick={confirmSize} />
            </div>
          )}
        </Card>

        {/* Voice */}
        <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
          <SectionHeader label={t('appearance.voiceSet')} />
          {(() => {
            const currentVoice = voicesets.find(v => v.id === data.soundset);
            const currentName = currentVoice?.name ?? `#${data.soundset}`;
            return (
              <div className="t-base" style={{ marginBottom: 8, display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{ color: T.textMuted }}>{t('appearance.voiceCurrent')}:</span>
                <span className="t-semibold">{currentName}</span>
                {currentVoice && (
                  <Button
                    icon={playingResref === currentVoice.resref ? 'stop' : 'volume-up'}
                    minimal
                    small
                    onClick={() => {
                      if (playingResref === currentVoice.resref) {
                        if (audioRef.current) audioRef.current.pause();
                        setPlayingResref(null);
                      } else {
                        playVoicePreview(currentVoice.resref);
                      }
                    }}
                  />
                )}
              </div>
            );
          })()}
          <InputGroup
            placeholder={t('appearance.voiceSearch')}
            value={voiceFilter}
            onValueChange={setVoiceFilter}
            small
            leftIcon="search"
            style={{ marginBottom: 8 }}
          />
          <div style={{ maxHeight: 300, overflowY: 'auto', border: `1px solid ${T.border}`, borderRadius: 3 }}>
            {voicesets.length === 0 ? (
              <div className="t-base t-center" style={{ padding: 12, color: T.textMuted }}>
                {t('appearance.voiceNone')}
              </div>
            ) : (
              (() => {
                const typeLabels: Record<number, string> = {
                  0: t('appearance.voiceTypePlayer'),
                  1: t('appearance.voiceTypeHenchman'),
                  2: t('appearance.voiceTypeNPC'),
                  3: t('appearance.voiceTypeNPC'),
                  4: t('appearance.voiceTypeCreature'),
                };

                return groupedVoicesets.map(({ typeKey, voices }) => (
                  <div key={typeKey}>
                    <div className="t-uppercase" style={{
                      padding: '4px 8px',
                      color: T.accent,
                      background: T.surface,
                      borderBottom: `1px solid ${T.border}`,
                      position: 'sticky',
                      top: 0,
                      zIndex: 1,
                    }}>
                      {typeLabels[typeKey] ?? `Type ${typeKey}`}
                    </div>
                    {voices.map(v => {
                      const isCurrent = v.id === data.soundset;
                      const isPending = pendingVoiceId === v.id;
                      const isHighlighted = isPending || (isCurrent && pendingVoiceId === null);
                      const isPlaying = playingResref === v.resref;
                      return (
                        <div
                          key={v.id}
                          onClick={() => setPendingVoiceId(v.id === data.soundset ? null : v.id)}
                          className="t-base"
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            padding: '4px 8px',
                            cursor: 'pointer',
                            background: isHighlighted ? T.accent + '22' : 'transparent',
                            borderBottom: `1px solid ${T.border}`,
                            gap: 6,
                          }}
                        >
                          <span style={{
                            flex: 1,
                            fontWeight: isHighlighted ? 600 : 400,
                            color: isHighlighted ? T.accent : T.text,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}>
                            {v.name}
                          </span>
                          <Button
                            icon={isPlaying ? 'stop' : 'play'}
                            minimal
                            small
                            loading={isPlaying}
                            onClick={(e: React.MouseEvent) => {
                              e.stopPropagation();
                              if (isPlaying) {
                                if (audioRef.current) audioRef.current.pause();
                                setPlayingResref(null);
                              } else {
                                playVoicePreview(v.resref);
                              }
                            }}
                          />
                        </div>
                      );
                    })}
                  </div>
                ));
              })()
            )}
          </div>
          {pendingVoiceId !== null && (
            <div style={{ marginTop: 8, display: 'flex', justifyContent: 'flex-end', gap: 6 }}>
              <Button
                small
                text={t('common.cancel')}
                onClick={() => setPendingVoiceId(null)}
              />
              <Button
                small
                intent="primary"
                text={t('actions.apply')}
                onClick={() => {
                  updateField({ soundset: pendingVoiceId });
                  setPendingVoiceId(null);
                }}
              />
            </div>
          )}
        </Card>

        {/* Equipment Display */}
        <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
          <SectionHeader label={t('appearance.equipment')} />
          <KVRow
            label={t('appearance.showHelmet')}
            value={
              <Switch
                checked={showHelmetInViewer}
                onChange={() => {
                  const next = !showHelmetInViewer;
                  setShowHelmetInViewer(next);
                  updateField({ never_draw_helmet: !next }, 'helm');
                }}
                style={{ marginBottom: 0 }}
              />
            }
          />
        </Card>

        {/* Wings & Tail */}
        <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
          <SectionHeader label={t('appearance.wingsAndTail')} />
          <div className="t-md">
            <KVRow
              label={t('appearance.wings')}
              value={
                <Popover
                  content={
                    <Menu style={{ maxHeight: 300, overflowY: 'auto' }}>
                      {wingOptions.map(opt => (
                        <MenuItem
                          key={opt.id}
                          text={opt.name}
                          active={data.wings === opt.id}
                          onClick={() => updateField({ wings: Number(opt.id) }, 'wings')}
                        />
                      ))}
                    </Menu>
                  }
                  placement="bottom-end"
                  minimal
                >
                  <Button
                    minimal
                    rightIcon="caret-down"
                    text={wingOptions.find(o => o.id === data.wings)?.name ?? `#${data.wings}`}
                    className="t-semibold"
                  />
                </Popover>
              }
            />
            <KVRow
              label={t('appearance.tail')}
              value={
                <Popover
                  content={
                    <Menu style={{ maxHeight: 300, overflowY: 'auto' }}>
                      {tailOptions.map(opt => (
                        <MenuItem
                          key={opt.id}
                          text={opt.name}
                          active={data.tail === opt.id}
                          onClick={() => updateField({ tail: Number(opt.id) }, 'tail')}
                        />
                      ))}
                    </Menu>
                  }
                  placement="bottom-end"
                  minimal
                >
                  <Button
                    minimal
                    rightIcon="caret-down"
                    text={tailOptions.find(o => o.id === data.tail)?.name ?? `#${data.tail}`}
                    className="t-semibold"
                  />
                </Popover>
              }
            />
            {data.tail > 0 || data.wings > 0 ? (() => {
              const tailCap = data.tail > 0 ? caps.tail : null;
              const wingsCap = data.wings > 0 ? caps.wings : null;
              const merged: PartTintCapability | undefined = (tailCap || wingsCap)
                ? {
                    has_tint_map: Boolean(tailCap?.has_tint_map || wingsCap?.has_tint_map),
                    channels: [0, 1, 2].map(i =>
                      Boolean(tailCap?.channels[i] || wingsCap?.channels[i])
                    ) as [boolean, boolean, boolean],
                  }
                : undefined;
              return (
                <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 8 }}>
                  <div className="t-base t-semibold" style={{ color: T.textMuted }}>{t('appearance.bodyTint')}</div>
                  {([1, 2, 3] as const).map(n => (
                    <ColorPicker
                      key={n}
                      label={t('appearance.bodyTintChannel', { n })}
                      value={(pendingBodyTint ?? data.tint_body)[`channel${n}`]}
                      onChange={updateBodyTintChannel(`channel${n}`)}
                      {...capProps(merged, (n - 1) as 0 | 1 | 2)}
                    />
                  ))}
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 6 }}>
                    <Button
                      small
                      text={t('appearance.matchSkinTone')}
                      onClick={() => { setPendingBodyTint(null); updateField({ tint_body_match_skin: true }); }}
                    />
                    {pendingBodyTint !== null && (
                      <div style={{ display: 'flex', gap: 6 }}>
                        <Button small text={t('common.cancel')} onClick={cancelBodyTint} />
                        <Button small intent="primary" text={t('actions.apply')} onClick={confirmBodyTint} />
                      </div>
                    )}
                  </div>
                </div>
              );
            })() : null}
          </div>
        </Card>
      </div>

      {/* Right: 3D Preview */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <CharacterViewer3D
          refreshKey={modelRefreshKey}
          refreshPart={partRefresh}
          tintHead={pendingTints?.tint_head ?? data.tint_head}
          tintHair={pendingTints?.tint_hair ?? data.tint_hair}
          tintBody={pendingBodyTint ?? data.tint_body}
          tintCloak={data.cloak_tint}
          tintArmor={data.armor_tint}
          height={liveHeight}
          girth={liveGirth}
          showHelmet={showHelmetInViewer}
        />
      </div>
    </div>
  );
}
