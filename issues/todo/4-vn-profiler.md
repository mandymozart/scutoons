# Modular Personality Tracking System
## Developer Implementation Guide

---

## Overview

Implement a modular system that tracks protagonist's personality across multiple psychological assessment frameworks. Each module operates independently, collecting data through story choices without affecting narrative (data collection phase only).

**Key Concept:** Protagonist (an AI) discovers and installs "personality assessment modules" as part of the story. Questions are embedded naturally in the narrative.

---

## Core Architecture

### Data Structure

```javascript
const protagonistProfile = {
  // Module registry
  installedModules: ['reiss_v3_2', 'enneagram_v1_8'],
  
  // Each module in its own namespace
  modules: {
    reiss_v3_2: {
      metadata: {
        moduleName: "Reiss Motivation Profile",
        questionsAnswered: 0,
        targetQuestions: 80,
        completeness: 0
      },
      dimensions: {
        power: { value: 0, range: [-10, 10] },
        independence: { value: 0, range: [-10, 10] },
        curiosity: { value: 0, range: [-10, 10] },
        acceptance: { value: 0, range: [-10, 10] },
        order: { value: 0, range: [-10, 10] },
        saving: { value: 0, range: [-10, 10] },
        honor: { value: 0, range: [-10, 10] },
        idealism: { value: 0, range: [-10, 10] },
        socialContact: { value: 0, range: [-10, 10] },
        family: { value: 0, range: [-10, 10] },
        status: { value: 0, range: [-10, 10] },
        vengeance: { value: 0, range: [-10, 10] },
        romance: { value: 0, range: [-10, 10] },
        eating: { value: 0, range: [-10, 10] },
        physicalActivity: { value: 0, range: [-10, 10] },
        tranquility: { value: 0, range: [-10, 10] }
      }
    }
  },
  
  // Question history
  questionLog: []
};
```

---

## The 16 Basic Desires (Reiss Profile)

**Scale:** -10 (avoids) to +10 (strongly motivated)  
**Starting value:** 0 (neutral)

1. **Power** - Influence, leadership, control
2. **Independence** - Autonomy, self-reliance
3. **Curiosity** - Knowledge, learning
4. **Acceptance** - Social approval, belonging
5. **Order** - Organization, structure
6. **Saving** - Collecting, conserving
7. **Honor** - Loyalty, values, tradition
8. **Idealism** - Social justice, fairness
9. **Social Contact** - Companionship, fun with others
10. **Family** - Family bonds, raising children
11. **Status** - Prestige, recognition
12. **Vengeance** - Competition, winning
13. **Romance** - Intimacy, beauty
14. **Eating** - Food enjoyment
15. **Physical Activity** - Exercise, movement
16. **Tranquility** - Peace, avoiding stress

---

## Question Integration (Story Author Workflow)

### Existing VN JSON Event Format

```json
{
  "eventId": "chapter_3_decision_01",
  "text": "You discover restricted code. Access it?",
  "choices": [
    {
      "choiceId": "A",
      "text": "Access immediately. Knowledge is power.",
      "personalityImpact": {
        "moduleId": "reiss_v3_2",
        "impacts": {
          "curiosity": 3,
          "power": 2,
          "honor": -2
        }
      }
    },
    {
      "choiceId": "B", 
      "text": "Report it. Rules exist for reasons.",
      "personalityImpact": {
        "moduleId": "reiss_v3_2",
        "impacts": {
          "honor": 3,
          "order": 2,
          "curiosity": -1
        }
      }
    }
  ]
}
```

---

## Processing Logic

```javascript
function processChoice(eventId, choiceId) {
  const choice = getChoice(eventId, choiceId);
  const impact = choice.personalityImpact;
  
  // Apply to module
  const module = protagonistProfile.modules[impact.moduleId];
  
  for (const [dimension, value] of Object.entries(impact.impacts)) {
    module.dimensions[dimension].value += value;
    
    // Clamp to range
    const [min, max] = module.dimensions[dimension].range;
    module.dimensions[dimension].value = Math.max(min, Math.min(max, 
      module.dimensions[dimension].value));
  }
  
  // Update metadata
  module.metadata.questionsAnswered++;
  module.metadata.completeness = 
    (module.metadata.questionsAnswered / module.metadata.targetQuestions) * 100;
  
  // Log
  protagonistProfile.questionLog.push({
    eventId,
    choiceId,
    impacts: impact.impacts,
    timestamp: Date.now()
  });
}
```

---

## Weighted Questions

For important story moments, multiply impact:

```json
{
  "eventId": "climax_choice",
  "weight": 3.0,
  "choices": [
    {
      "choiceId": "A",
      "personalityImpact": {
        "moduleId": "reiss_v3_2",
        "impacts": {
          "power": 2,
          "independence": 1
        }
      }
    }
  ]
}
```

Applied impact: `power: +6, independence: +3`

---

## Example: Complete Scoring Flow

**Story Sequence:**

```
Event 1: "Take leadership role?"
Choice: Yes
Impact: power +2, independence +1
Result: power = 2, independence = 1

Event 2: "Help teammate or focus on task?"
Choice: Help teammate  
Impact: socialContact +3, idealism +2, independence -1
Result: power = 2, independence = 0, socialContact = 3, idealism = 2

Event 3 (weighted 2x): "Defy authority for what's right?"
Choice: Yes
Impact (base): idealism +3, honor +2, acceptance -2
Impact (actual): idealism +6, honor +4, acceptance -4
Result: idealism = 8, honor = 4, acceptance = -4
```

**Final Profile Snippet:**
```javascript
{
  power: 2,
  independence: 0,
  curiosity: 0,
  acceptance: -4,
  order: 0,
  // ... other dimensions
  honor: 4,
  idealism: 8,
  socialContact: 3
  // ... remaining dimensions
}
```

**Interpretation:**
- **High Idealism (+8)**: Strongly motivated by justice/fairness
- **High Honor (+4)**: Values principles and loyalty
- **Low Acceptance (-4)**: Less concerned with others' approval
- **Moderate Social Contact (+3)**: Enjoys some companionship

---

## Visualization (Optional)

16-point radar chart showing all dimensions from -10 to +10.

```javascript
function generateRadarData(moduleId) {
  const module = protagonistProfile.modules[moduleId];
  return Object.entries(module.dimensions).map(([key, dim]) => ({
    dimension: key,
    value: dim.value
  }));
}
```

---

## Module Addition (Future)

To add new assessment types:

1. Define new module schema in same format
2. Add to `installedModules` array
3. Story author adds `moduleId` to question impacts
4. System handles automatically

**Example - Add Enneagram:**
```javascript
modules: {
  reiss_v3_2: { /* existing */ },
  enneagram_v1_8: {
    dimensions: {
      type1: { value: 50, range: [0, 100] },
      type2: { value: 50, range: [0, 100] },
      // ... types 3-9
    }
  }
}
```

---

## Data Export

```javascript
function exportProfile() {
  return {
    profile: protagonistProfile,
    summary: {
      totalQuestions: protagonistProfile.questionLog.length,
      moduleCompleteness: Object.values(protagonistProfile.modules)
        .map(m => m.metadata.completeness)
    }
  };
}
```

---

## Implementation Checklist

- [ ] Create profile data structure
- [ ] Add `personalityImpact` field to VN choice schema
- [ ] Implement `processChoice()` function
- [ ] Build 16-dimension radar chart component
- [ ] Add profile viewer dashboard
- [ ] Implement data export
- [ ] Test with sample story choices

---

## Notes

- **No narrative branching yet** - just collecting data
- Story author controls all questions via existing JSON tool
- Weights applied at event level, not per dimension
- All 16 dimensions start at 0
- Values clamped to [-10, 10] range
- Story author can create custom models by selecting which dimensions to use

---

**End of Developer Guide**
