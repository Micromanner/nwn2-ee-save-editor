// Simple test to verify our vital statistics persistence fix logic
// Converted to JavaScript for easier testing

// CharacterStats interface (TypeScript-style comment for reference)

// Simulate the fix we implemented
function extractBaseTotal(obj, statType, localOverrides) {
  // Check for local overrides first (for persistence across tab switches)
  const overrideKey = statType === 'ac' ? 'armorClass' : statType;
  const localOverride = localOverrides[overrideKey];
  
  if (typeof obj === 'number') {
    // Simple number - assume it's total, no editable base
    const baseValue = localOverride && typeof localOverride === 'object' && 'base' in localOverride 
      ? localOverride.base 
      : 0;
    return { base: baseValue, total: obj };
  }
  
  if (typeof obj === 'object' && obj !== null) {
    const objData = obj;
    
    let base = 0;
    let total = 0;
    
    // Get total value
    total = (typeof objData.total === 'number' ? objData.total : 
            typeof objData.value === 'number' ? objData.value : 0);
    
    // Check for local override base value first
    if (localOverride && typeof localOverride === 'object' && 'base' in localOverride) {
      base = localOverride.base;
    } else {
      // Get base value from backend based on stat type
      switch (statType) {
        case 'ac':
          // Natural Armor comes from components.natural (from NaturalAC GFF field)
          const components = objData.components;
          base = (typeof components?.natural === 'number' ? components.natural : 0);
          break;
        case 'initiative':
          // Initiative base is misc_bonus (editable miscellaneous bonus)
          base = (typeof objData.misc_bonus === 'number' ? objData.misc_bonus : 0);
          break;
        case 'fortitude':
        case 'reflex':
        case 'will':
          // Saving throws base comes from the 'base' field in the save object
          base = (typeof objData.base === 'number' ? objData.base : 0);
          break;
      }
    }
    
    return { base, total };
  }
  
  return { base: 0, total: 0 };
}

// Test scenario: User has edited AC base to 5, then switches tabs
console.log('=== Vital Statistics Persistence Fix Test ===');

// Simulate backend data (fresh from API)
const backendAcData = {
  total: 15,
  components: { natural: 0 } // Original backend value
};

// Simulate local overrides (user had edited AC base to 5)
const localOverrides = {
  armorClass: { base: 5, total: 15 }
};

// Test WITHOUT our fix (old behavior)
const oldResult = extractBaseTotal(backendAcData, 'ac', {});
console.log('WITHOUT fix (should lose user edit):', oldResult);
// Expected: { base: 0, total: 15 } - user's edit is lost!

// Test WITH our fix (new behavior)
const newResult = extractBaseTotal(backendAcData, 'ac', localOverrides);
console.log('WITH fix (should preserve user edit):', newResult);
// Expected: { base: 5, total: 15 } - user's edit is preserved!

console.log('Fix verification:', newResult.base === 5 ? 'PASS ✓' : 'FAIL ✗');