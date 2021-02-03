# -*- coding: utf-8 -*-

# branch test!!!

import copy
from random import randint
from objs.item import getItem
from objs.object import Object
from objs.skill import MUGONG
from objs.oneitem import Oneitem, ONEITEM
from objs.script import Script, SCRIPT
from objs.config import MAIN_CONFIG
from lib.hangul import *
from lib.func import *
from include.define import *
import math

class Body(Object):
    ItemUseLevel = \
    { '투구':	'투    구', '왕관':	  '   관   ', '머리':	'머    리',
      '귀걸이':	'귀 걸 이', '목걸이': '목 걸 이', '어깨':	'어    깨',
      '상의':	'상    의', '하의':	  '하    의', '장신구':	'장 신 구',
      '갑옷':	'갑    옷', '허리':	  '허    리', '팔찌':	'팔    찌',
      '장갑':	'장    갑', '반지':	  '반    지', '슬호':	'슬    호',
      '신발':	'신    발', '무기':	  '무    기', '기타':	'기    타', }
    
    ItemLevelList = \
        ['투구', '왕관', '머리', '귀걸이', '목걸이', '어깨', '상의', '하의', '장신구',
         '갑옷', '허리', '팔찌', '장갑', '반지', '슬호', '신발', '무기', '기타']

    strBar = [ \
        '[37m━━━━━━━━━━[37m',
        '[31m━[37m━━━━━━━━━[37m',
        '[31m━━[37m━━━━━━━━[37m',
        '[31m━━━[37m━━━━━━━[37m',
        '[33m━━━━[37m━━━━━━[37m',
        '[33m━━━━━[37m━━━━━[37m',
        '[33m━━━━━━[37m━━━━[37m',
        '[32m━━━━━━━[37m━━━[37m',
        '[32m━━━━━━━━[37m━━[37m',
        '[32m━━━━━━━━━[37m━[37m',
        '[32m━━━━━━━━━━[37m']

    difficulty = [
        [2, 1.3, 1, 1],
        [3.2, 1.88, 2, 2],
        [5.12, 2.7333, 3, 3],
        [8.19, 3.9632, 4, 4],
        [13.11, 5.7467, 5, 5],
        [20.97, 8.3327, 6.5, 6.5],
        [33.55, 12.0823, 9, 9],
        [53.69, 17.5194, 12.5, 12.5],
        [85.9, 25.4031, 16, 16],
    ]

    skillLv = {
        "초급" : 1,
        "중급" : 2,
        "상급" : 3,
        "고급" : 4,
        "특급" : 5,
        "절정" : 6,
        "초절정" : 7
    }
    
    skillLvName = [
        "초급",
        "중급",
        "상급",
        "고급",
        "특급",
        "절정",
        "초절정"
    ]

    skillLvType = [
        "초급",
        "중급",
        "상급",
        "고급",
        "특급",
        "절정",
        "초절정",
        "회복",
        "방어",
        "기타"
    ]


    skillLvMap = { }
    
    def __init__(self):
        Object.__init__(self)
        self.act = ACT_STAND
        self.tick = 0
        self.skill = None
        self.lastskill = None
        self.attpower = 0
        self.armor = 0
        self.dex = 0
        self._str = 0
        self._dex = 0
        self._arm = 0
        self._mp = 0
        self._maxmp = 0
        self._hp = 0
        self._maxhp = 0
        self._hit = 0
        self._miss = 0
        self._critical = 0
        self._criticalChance = 0
        self._magicChance = 0
        self._exp = 0
        self.weaponItem = None

    def getStr(self):
        if self._str + self['힘'] < 0:
            return 0
        return self._str + self['힘']
        
    def getDex(self):
        if self._dex + self['민첩성'] < 0:
            return 0
        return self._dex + self['민첩성']
        
    def getArm(self):
        if self._arm + self['맷집'] < 0:
            return 0
        alpha = 0
        if self['맷집상승'] != '':
            alpha = 1000
        return self._arm + self['맷집'] + alpha
        
    def getMp(self):
        if self._mp != 0:
            mp = self['내공'] + (self['내공'] * self._mp) // 100
            return mp
        return self['내공']
        
    def getMaxMp(self):
        if self._maxmp != 0:
            # mp = self['최고내공'] + (self['최고내공'] * self._maxmp) // 100
            # limit max 
            mp = self['최고내공'] + self._maxmp
            return mp
        return self['최고내공']
        
    def getHp(self):
        return self['체력']
        
    def getMaxHp(self):
        #h = self['최고체력'] + (self.getArm() - (self['레벨'] + 14)) * 30
        h = self['최고체력'] + (self.getArm()) * 30
        if self._maxhp != 0:
            # return h + (h * self._maxhp) // 100
            # limit max 
            return h + self._maxhp
        return h

    def getHit(self):
        if self['명중'] == '':
            self['명중'] = 0
        if self._hit != 0:
            return self['명중'] + self._hit
        return self['명중']
        
    def getCritical(self):
        if self['필살'] == '':
            self['필살'] = 0
        if self._critical != 0:
            return self['필살'] + self._critical
        return self['필살']

    def getCriticalChance(self):
        if self['운'] == '':
            self['운'] = 0
        if self._criticalChance != 0:
            return self['운'] + self._criticalChance
        return self['운']
            
    def getMiss(self):
        if self['회피'] == '':
            self['회피'] = 0
        if self._miss != 0:
            return self['회피'] + self._miss
        return self['회피']
        
    def getBonusExp(self):
        return self._exp

    def getBonusMagicChance(self):
        return self._magicChance

    def setTarget(self, ob):
        self.act = ACT_FIGHT
        ob.act = ACT_FIGHT
        if ob not in self.target:
            self.target.append(ob)
        
    def clearTarget(self, ob = None):
        
        if ob != None:
            if ob in self.target:
                self.target.remove(ob)
            if self in ob.target:
                ob.target.remove(self)
            if self.act == ACT_FIGHT and len(self.target) == 0:
                self.act = ACT_STAND
                self.dex = 0
                self.stopSkill()
            if ob.act == ACT_FIGHT and len(ob.target) == 0:
                ob.act = ACT_STAND
                self.dex = 0
                ob.stopSkill()
            return
        target = copy.copy(self.target)
        for ob in target:
            self.clearTarget(ob)
        
    def getSkill(self, sName):
        if self.lastskill != None:
            if sName == self.lastskill.name:
                self.skill = self.lastskill
                return self.skill
            else:
                del self.lastskill
        self.skill = MUGONG[sName].clone()
        #self.lastskill = self.skill
        return self.skill
        
    def stopSkill(self):
        if self.skill != None:
            self.lastskill = self.skill
            self.skill = None
        
    def clearSkills(self):
        self.stopSkill()
        ss = copy.copy(self.skills)
        for s in ss:
            self.skills.remove(s)
        
    def isMovable(self):
        if self.act == ACT_FIGHT or self.act == ACT_REST:
            return False
        return True

    def init_body(self):
        self.set('레벨', 1)
        self.set('체력', 450)
        self.set('최고체력', 450)
        self.set('힘', 15)
        self.set('맷집', 15)
        self.set('민첩성', 0)
        self.set('은전', 100000)
        self.set('금전', 0)
        self.set('내공', 18)
        self.set('최고내공', 18)
        self.set('나이', 18)
        self.set('나이오름틱', 0)
        self.set('현재경험치', 0)
        self.set('1 숙련도', 0)
        self.set('2 숙련도', 0)
        self.set('3 숙련도', 0)
        self.set('4 숙련도', 0)
        self.set('5 숙련도', 0)
        self.set('1 숙련도경험치', 0)
        self.set('2 숙련도경험치', 0)
        self.set('3 숙련도경험치', 0)
        self.set('4 숙련도경험치', 0)
        self.set('5 숙련도경험치', 0)
        self.set('힘경험치', 0)
        self.set('민첩성경험치', 0)
        self.set('0 성격플킬', 0)
        self.set('1 성격플킬', 0)
        self.set('2 성격플킬', 0)
        self.set('무공숙련도', '')
        self.set('무공이름', '')
        self.set('보험료', 0)
        
    def GetHPString(self):
        scripts = SCRIPT['사용자스크립']
        cnt = len(scripts)
        if cnt == 0:
            return ''
        s = scripts[(cnt - 1) - ((cnt - 1) * self.getHp()) // self.getMaxHp()]
        s = self['이름'] + postPosition(s, self['이름'])
        return s
        
    def getItemWeight(self):
        w = 0
        for obj in self.objs:
            if obj.checkAttr('아이템속성', '출력안함'):
                continue
            w += getInt(obj['무게'])
        return w
        
    def getItemCount(self):
        c = 0
        for item in self.objs:
            if item.checkAttr('아이템속성', '출력안함'):
                continue
            c += 1
        return c
        
    def getInvenItemCount(self):
        c = 0
        for item in self.objs:
            if item.checkAttr('아이템속성', '출력안함') or item.inUse:
                continue
            c += 1
        return c
    
    def getTotalExp(self):
        cc = self['레벨']
        c =(((cc * cc) // 3) + 30) * (cc + 4)
        n = MAIN_CONFIG['최고경험치']
#        if cc >= 1800:
#            return cc * 200000 
#        if  n < 1: 
#            n = MAX_INT
        if c < 1:
            c = 1
#        if c > n:
#            c = n
        if c > n:
            N = 200
            if self['레벨'] >= 3000:
                N = int(self['레벨'] // 10)
            c1 = getInt(self['레벨'])
            c2 = c1 + 200

            a = ((c2 * c2) // 3) + 30
            b = (a * (c2 - c1)) // 100
            c = (a + b) * N
        return c

    def addArm(self, arm):
        self['맷집경험치'] += arm
        c = (self['맷집'] - 10) * 20
        if self['맷집경험치'] >= c:
            self['맷집경험치'] = 0
            self['맷집'] += 1
            self.sendLine(MAIN_CONFIG['맷집증가스크립'])    

    def addExp(self, exp):
        self['현재경험치'] += exp
        t_exp = self.getTotalExp()
        if self['현재경험치'] >= t_exp:
            #self['현재경험치'] -= t_exp
            self['현재경험치'] = 0
            self['레벨'] += 1
            self.levelUp()
        
    def addStr(self, str, check = True):
        self['힘경험치'] += str
        c = (self['힘'] - 10) * 20
        if check and self['힘경험치'] >= c:
            self['힘경험치'] = 0
            self['힘'] += 1
            self.sendLine(MAIN_CONFIG['힘증가스크립'])
    
    def addDex(self, dex):
        self['민첩성경험치'] += dex
        c = (self['민첩성'] + 4) * 8
        if self['민첩성경험치'] >= c:
            self['민첩성경험치'] = 0
            if self['민첩성'] < MAIN_CONFIG['민첩성최고수치']:
                self['민첩성'] += 1
                self.sendLine(MAIN_CONFIG['민첩성증가스크립'])

    def addAnger(self):
        anger = getInt(self['분노'])
        
        if anger >= 600:
            return
        anger += 1
        if anger == 100:
            self.sendFightScript('당신이 갑자기 [1;40;31m괴성[0;40;37m을 지르며 [1;40;31m난동[0;40;37m을 부립니다. \'끄오오오오~~\'')
            self.sendFightScriptRoom('%s 갑자기 [1;40;31m괴성[0;40;37m을 지르며 [1;40;31m난동[0;40;37m을 부립니다. \'끄오오오오~~\'' % self.han_iga())
        self['분노'] = anger
        
    def levelUp(self):
        self.sendLine(MAIN_CONFIG['레벨증가스크립'])
        hpUp = randint(0, 9) + 25;
        self['최고체력'] += hpUp
        self['맷집'] += 1
        self['체력'] = self.getMaxHp()
        self['내공'] = self.getMaxMp()
        self.sendLine('☞ 체력 상승 ▷ %d〔%d〕, 맷집 상승 ▷ 1〔%d〕' % \
        ( hpUp, self['최고체력'], self['맷집']))
        if getInt(self['전직']) >= 2:
            return
        if self['레벨'] >= 2000:
            self['특성치'] += 1
        else:
            if self['레벨'] % 10 == 0:
                self['특성치'] += 1

    def loadSkillUp(self):
        self.skillMap = {}
        lines = self['무공숙련도']
        if lines == '':
            return
        for line in lines:
            words = line.split()
            self.skillMap[words[0]] = (int(words[1]), int(words[2]))
            
    def buildSkillUp(self):
        msg = []
        for sup in self.skillMap:
            msg.append('%s %d %d' % (sup, self.skillMap[sup][0], self.skillMap[sup][1]))
        self['무공숙련도'] = msg
        
    def skillUp(self, s = None):
        if s == None:
            s = self.skill

        if s.name not in self.skillMap:
            self.skillMap[s.name] = (1, 0)
        
        s1 = self.skillMap[s.name][0]
        s2 = self.skillMap[s.name][1]
        s2 += 1
        
        up = False
        lv1 = 1
        if s.name in Body.skillLvMap:
            lv1 = self.skillLvMap[s.name]
        
        # test only!!!
        
        exp1 = 10000 * lv1
        div = 1
        if s['종류'] != '공격':
            div = 10

        if (s1 == 10 and s2 > exp1 // div) or (s1 == 11 and s2 > exp * 2 // div):
            sLv = self['무공달성레벨']
            if sLv == '':
                sLv = self.skillLvName[0]
                self['무공달성레벨'] = sLv 

            lv2 = self.skillLv[sLv]
            if lv1 <= lv2:
                up = True

        # 임시로 안오르게 조치
        #if s['종류'] == '공격':
        #    up = False
            
        if (s1 < 10 and s2 >= s['확률증가']) or up == True:
            s1 += 1
            s2 = 0
            if s1 > 12:
                return
            else:
                self.sendLine('[1m당신이 무공을 펼치기위한 진기집성이 수월해 지는것을 느낍니다.[0m[40m[37m')
                if s1 == 12:
                    self.checkSkillLvUp()

        self.skillMap[s.name] = (s1, s2)
        
    def checkSkillLvUp(self):
        lv = self.skillLv[ self['무공달성레벨'] ]
        count = 0
        for name in self.skillMap:
            s1, s2 = self.skillMap[name]
            if name in self.skillLvMap and self.skillLvMap[name] == lv and s1 == 12:
                count += 1
        if count >= 12:
            self['무공달성레벨'] = self.skillLvName[lv]
            self.sendLine('!!!!!!!!!!!!!')

    def weaponSkillUp(self, n = 1):
        type = self.getWeaponType()
        buf1 = '%d 숙련도' % type
        buf2 = '%d 숙련도경험치' % type
        c = getInt(self[buf1])
        cc = getInt(self[buf2])
        cc += n
        self[buf2] = cc
        c = (c + 5 ) * 7
        if cc >= c:
            self[buf1] += 1
            self[buf2] = 0
            self.sendLine(MAIN_CONFIG['숙련도증가스크립'])
            
    def getAttackFailScript(self, mob):
        s = SCRIPT[self.getWeaponFightType() + '전투실패스크립']
        s = s[randint(0, len(s) - 1)]
        return self.makeFightScript(s, mob)
        
    def getAttackScript(self, mob, dmg, c1, c2):
        s = SCRIPT[self.getWeaponFightType() + '전투스크립']
        if len(s) == 0:
            return '버그버그버그버그버그버그버그버그버그버그신고하셈신고하셈'
        s = s[randint(0, len(s) - 1)]
        return self.makeFightScript(s, mob)
        
    def getSkillChance(self, mob):
        l1 = self['레벨']
        l2 = mob['레벨']
        
        # limit attack level
        if (l2 - l1) >= MAIN_CONFIG['최대사냥레벨차이']:
            return -1        
        
        if self.skill != None:
            CHANCE = self.skill['확률']
            #무공숙련도 추가필요
            if self.skill.name in self.skillMap:
                CHANCE += self.skillMap[self.skill.name][0] * MAIN_CONFIG['기술확률배수']
        else:
            CHANCE = 100
        bonus = self.getHit() * float(MAIN_CONFIG['명중확률'])
        bonus -= mob.getMiss() * float(MAIN_CONFIG['회피확률']) 
            
        bonus_chance = 0

        if self.skill.name in self.skillMap:
            s1 = self.skillMap[self.skill.name][0]
            if s1 == 11:
                bonus_chance = 10
            elif s1 == 12:
                bonus_chance = 20

        return CHANCE - (((l2-l1)+90)//3) + bonus + bonus_chance
        
    def getAttackChance(self, mob):
        l1 = self['레벨']
        l2 = mob['레벨']
        
        # limit attack level
        if (l2 - l1) >= MAIN_CONFIG['최대사냥레벨차이']:
            return -1
        
        CHANCE = 100
        bonus = self.getHit() * float(MAIN_CONFIG['명중확률'])
        bonus -= mob.getMiss() * float(MAIN_CONFIG['회피확률']) 
        return CHANCE - (((l2-l1)+90)//3) + bonus
        
    def getAttackPoint(self, mob):
        item = self.getWeapon()
        s1 = 0
        from objs.player import is_player
        if is_player(self): 
            if item != getItem('주먹'):
                s1 = getInt(item['기량'])
        s2 = getInt(self['%d 숙련도' % self.getWeaponType()])
        if self['숙련도상승'] != '':
            s2 += 2000 
        ss = s1 - s2;
        if ss < 0:
            ss = 0
        c1 = self.getStr() * 2
        if is_player(self): 
            #c1 += math.sqrt( self.getStr() * self.getMaxMp() )
            c1 += self.getMaxMp() // 5
        c2 = self.getAttPower() - ss
        m1 = (c1 + c2) - (mob.getArm() + mob.getArmor())
        if m1 < 1:
            m1 = 1
        m = m1
        #print 's1=%d, s2=%d, ss=%d, c1=%d, c2=%d, m1=%d, m=%d' % (s1, s2, ss, c1, c2, m1, m)
        c1 = int(m * 0.80)
        c2 = int(m * 1.20)
        
        s1 = c2 - c1 + 1
        #print 's1=%d, c1=%d, c2=%d' % (s1, c1, c2)
        if s1 < 1:
            s1=1;
    
        m = randint(0, s1 - 1) + c1
        
        if m < 1:
            m = 1
        #print 'c1=%d, c2=%d, m=%d' % (c1, c2, m)
        return int(m), c1, c2
        
    def getArmor(self):
        return self.armor
        
    def getAttPower(self):
        return self.attpower
        
    def getSkillPoint(self, mob):
        m, c1, c2 = self.getAttackPoint(mob)
        f = float(self.skill['타격률'])
        
        if f <= 0:
            f = 0.1
        m += m * f
        m = int(m)

        bonus_chance = 0
        bonus_dmg = 1

        if self.skill.name in self.skillMap:
            s1 = self.skillMap[self.skill.name][0]
            if s1 == 11:
                bonus_chance = 10
                bonus_dmg = 1.3
            elif s1 == 12:
                bonus_chance = 20
                bonus_dmg = 1.5

        chance = self.getCriticalChance() * float(MAIN_CONFIG['운확률']) + bonus_chance
        bonus = 1
        if chance > randint(0, 100):
            bonus = self.getCritical() * float(MAIN_CONFIG['필살배수'])
            #if self.env['난이도'] != None and self.env['난이도'] != '':
            #    bonus = bonus // 2
            if bonus < 1:
                bonus = 1
        return int(m * bonus * bonus_dmg)
        
    def getWeaponType(self):
        return self.getWeapon()['무기종류']
        
    def getWeaponFightType(self):
        #무기 타입이 필요함
        return self.getWeapon()['전투스크립']
        
    def getWeapon(self):
        if self.weaponItem != None:
            return self.weaponItem
        return getItem('주먹')
        
    def makeFightScript(self, line, mob, weapon = None):
        if type(line) == list:
            line = '\r\n'.join(line)
        if mob == None:
            mName = ''
        else:
            mName = mob.getNameA()
            
        if weapon == None:
            m = self.getWeapon()
            if m == getItem('주먹'):
                mstr = '[36m주먹[37m'
            else:
                mstr = m.getNameA()
        else:
            mstr = weapon.getNameA()
            
        buf1 = line.replace('[공]', '당신')
        buf1 = buf1.replace('[방]', mName)
        buf1 = buf1.replace('[무]', mstr)
        buf1 = postPosition1(buf1)
        buf1 = postPosition1(buf1)
        buf1 = postPosition1(buf1)
        
        buf2 = line.replace('[공]', self.getNameA())
        buf2 = buf2.replace('[방]', '당신')
        buf2 = buf2.replace('[무]', mstr)
        buf2 = postPosition1(buf2)
        buf2 = postPosition1(buf2)
        buf2 = postPosition1(buf2)
        
        buf3 = line.replace('[공]', self.getNameA())
        buf3 = buf3.replace('[방]', mName)
        buf3 = buf3.replace('[무]', mstr)
        buf3 = postPosition1(buf3)
        buf3 = postPosition1(buf3)
        buf3 = postPosition1(buf3)
        
        return buf1, buf2, buf3
    
    def sendLine(self, line):
        return
    
    def sendRoom(self, line):
        return
        
    def writeRoom(self, line):
        return
        
    def sendRoomFightScript(self, line):
        return
        
    def lpPrompt(self):
        return
        
    def checkDefenceSkill(self):
        skills = copy.copy(self.skills)
        msg = '\r\n'
        autoSkill = []
        for s in skills:
            s.start_time -= 1
            if s.start_time < 0:
                self.skills.remove(s)
                self._str -= s._str
                self._dex -= s._dex
                self._arm -= s._arm
                self._mp -= s._mp
                self._maxmp -= s._maxmp
                buf1, buf2, buf3 = self.makeFightScript(s['무공해제스크립'], None)
                msg += buf1 + '\r\n'
                autoSkill.append(s.name)
                del s
        if len(msg) != 2:
            self.write(msg)
            self.lpPrompt()
            self.sendFightScriptRoom(buf3)
            
        if len(autoSkill) != 0 and '자동무공' in self.alias:
            a = self.alias['자동무공']
            askill = a.split(';')
            for s in autoSkill:
                if s in askill:
                    self.do_command('%s 시전' % s)

    def checkItemSkill(self):
        m = self.getWeapon()
        if m == getItem('주먹'):
            return
        mlist = m['무공이름']
        if len(mlist) == 0:
            return
        mName = m['이름']
        if m['이름'] not in self.itemSkillMap:
            self.itemSkillMap[mName] = 1
        else:
            self.itemSkillMap[mName] += 1
        p = self.itemSkillMap[mName]
        for s in mlist:
            words = s.split()
            sName = words[0]
            if sName in self.skillList:
                continue
            type = words[1]
            if type != '정사':
                if self['성격'] != type and self['성격'] != '기인' and self['성격'] != '선인':
                    continue
            n1 = int(words[2])
            count = self.itemSkillMap[mName]
            if count < n1:
                continue
            n2 = int(words[3])
            if p % n2 != 0:
                continue
            n3 = int(words[4])
            
            r = randint(0, 99)
            #print n1, n2, n3, r
            if  count < 2500000 and r > n3:
                continue

            self.skillList.append(sName)
            self.itemSkillMap[mName] = 0
            self.sendLine('\r\n[1m[40m[37m당신이 『[1m[40m[32m%s[1m[40m[37m』의 무공 구결을 깨우치기 시작합니다. \'ΔΨΞλΟ~\'[0m[40m[37m\r\n' % sName)
            self.sendRoom('[1m[40m[37m%s 『[1m[40m[32m%s[1m[40m[37m』의 무공 구결을 깨우치기 시작합니다. \'ΔΨΞλΟ~\'[0m[40m[37m' % (self.getNameA(), sName))
            attr = m['아이템속성']
            for at in attr:
                if at.find('무공배운후소멸') == 0:
                    m.inUse = False
                    self.armor -= getInt(m['방어력'])
                    self.attpower -= getInt(m['공격력'])
                    m.env = None
                    self.weaponItem = None 
                    self.objs.remove(m) 
                    del m
                    break
            break
            
    def loadSkills(self):
        ss = self['방어무공시전']
        if ss == '':
            return

        if type(ss) != list:
            ss = [ss]
        for line in ss:
            words = line.split()
            print(line, line)
            s = MUGONG[words[0]]
            s = copy.copy(s)
            self.skills.append(s)
            s.start_time = int(words[1])
            self._str += s._str
            self._dex += s._dex
            self._arm += s._arm
            self._mp += s._mp
            self._maxmp += s._maxmp
    
    def buildSkills(self):
        self['방어무공시전'] = ''
        for s in self.skills:
            buf = '%s %d' % (s.name, s.start_time)
            self.setAttr('방어무공시전', buf)
            
    def loadSkillList(self):
        self.skillList = self['무공이름']
        lines = self['무공이름수련리스트']
        for line in lines:
            words = line.split()
            self.itemSkillMap[words[0]] = int(words[1])
        
    def buildSkillList(self):
        msg = []
        for s in self.skillList:
            msg.append(s)
        self['무공이름'] = msg
        
        msg = []
        for s in self.itemSkillMap:
            msg.append('%s %s' % (s, self.itemSkillMap[s]))
        self['무공이름수련리스트'] = msg

    def unwearAll(self):
        self.attpower = 0
        self.armor = 0
        self._str = 0
        self._dex = 0
        self._arm = 0
        self._mp = 0
        self._maxmp = 0
        self._hp = 0
        self._maxhp = 0
        self._hit = 0
        self._miss = 0
        self._critical = 0
        self._criticalChance = 0
        self._magicChance = 0
        self._exp = 0
        for item in self.objs:
            if item.inUse == True:
                item.inUse = False
                if item['종류'] == '무기':
                    self.weaponItem = None

    def dropAllItem(self):
        p = self.getInsureCount()
        self.insure = 0
        nCnt = {}
        nFail = {}
        c = 0
        objs = copy.copy(self.objs)
        for item in objs:
            if p > 0 and item.checkAttr('아이템속성', '보험적용안됨') == False:
                self.insure += 1
                continue
            if item.checkAttr('아이템속성', '줄수없음'):
                continue
            if item.checkAttr('아이템속성', '버리지못함'):
                continue
            if item.checkAttr('아이템속성', '출력안함'):
                continue
            if item.checkAttr('아이템속성', '단일아이템'):
                ONEITEM.drop2(item.index, self['이름'])
            item.inUse = False
            self.objs.remove(item)
            c += 1
            if self.env.getItemCount() < 50:
                self.env.insert(item)
                item.drop()
                nc = 0
                post = item.han_obj()
                try:
                    nc, post = nCnt[item['이름']]
                except:
                    nCnt[item['이름']] = (0, post)
                nCnt[item['이름']] = (nc + 1, post)
            else:
                nc = 0
                try:
                    nc = nFail[item.get('이름')]
                except:
                    nFail[item.get('이름')] = 0
                nFail[item.get('이름')] = nc + 1
                del item
        self.decInsureCount()
        if c == 0:
            return
            
        for name in nCnt:
            nc = nCnt[name][0]
            post = nCnt[name][1]
            if nc == 1:
                self.sendLine('당신이 [36m' + post + '[37m 떨어뜨립니다.')
            else:
                self.sendLine('당신이 [36m' + name + '[37m %d개를 떨어뜨립니다..' % nc)
        for name in nFail:
            nc = nFail[name]
            if nc == 1:
                self.sendLine('당신이 [36m' + post + '[37m 떨어뜨리자 바로 부서집니다.')
            else:
                self.sendLine('당신이 [36m' + name + '[37m %d개를 떨어뜨리자 바로 부서집니다.' % nc)
        
        
    def decInsureCount(self):
        p = self['보험료']
        c1 = self['레벨'] * MAIN_CONFIG['보험료단가']
        c2 = c1 * MAIN_CONFIG['보험출장률'] // 100
        p -= c2
        if p < 0:
            p = 0
        self['보험료'] = p
        
    def getInsureCount(self):
        return getInt(self['보험료']) // (self['레벨'] * MAIN_CONFIG['보험료단가'])
        
    def addFollow(self, f):
        self.follow = f
        f.addFollower(self)
        self.sendLine('당신은 %s 따라다니기 시작합니다.' % f.han_obj())
        
    def delFollow(self, other = False):
        if self.follow != None:
            if other == True:
                self.sendLine('')
            self.sendLine('당신이 %s 따라다니는 것을 그만둡니다.' % self.follow.han_obj())
            if other == True:
                self.lpPrompt()
            self.follow.delFollower(self)
            self.follow = None
        
    def addFollower(self, f):
        if f not in self.follower:
            self.follower.append(f)
            self.sendLine('\r\n%s 당신을 따라다니기 시작합니다.' % f.han_iga())
            self.lpPrompt()
            
    def delFollower(self, f = None, noPrompt = False):
        if f != None:
            if f in self.follower:
                self.follower.remove(f)
                self.sendLine('\r\n%s 당신과 따라다니는 것을 그만둡니다.' % f.han_iga())
                self.lpPrompt()
            return
        fs = copy.copy(self.follower)
        for f in fs:
            f.delFollow(True)
        self.follower = []
        
    def recoverDemage(self, dmg):
        t = 0
        if dmg == 0:
            return 0

        arm=0
        hp = self.get('최고체력')
        dd1 = float(hp)*0.1
        dd2 = float(hp) * (float(MAIN_CONFIG['맷집증가비율'])//100)
        if dmg > dd2:
            arm = 1
            dmg -=dd2
            if dmg>0:
                if dd2>0:
                    dd3 = int(dmg//dd2)
                else:
                    dd3 = int(dmg)
                arm += dd3
            self.addArm(arm)

        for s in self.skills:
            if s['계열'] == '전투회복':
                p = int(s['회복능력'].split()[1])
                if self.env['난이도'] != '' and self.env['난이도'] > 0:
                    p = p // 2
                r = dmg * p // 100
                t += r
                if self.checkConfig('수련모드') == False:
                    buf1, buf2, buf3 = self.makeFightScript(s['회복스크립'], None)
                    self.sendLine(buf1 + ' ([1;32m+ %d[0;37m)' % r)
        return t
        
    def minusHP(self, demage, mode = True, who = None):
        cc = self.get('체력')
        cc -= demage

        if cc <= 0:
            self.set('체력', 0)
            self.die(mode)
            return True
        self.set('체력', cc)
        return False

    def minusMP(self, demage, mode = True, who = None):
        cc = self.get('내공')
        cc -= demage

        if cc <= 0:
            self.set('내공', 0)
        self.set('내공', cc)
        return False

    def checkVision(self, skill):
        line = self['비전수련']
        if line == '':
            return
        if skill == None:
            return
        if skill.name not in line:
            return
        var = line.split()
        if len(var) == 1:
            p = 0
        else:
            p = int(var[1])
        n1 = 10
        n2 = 10
        
        n3 = 1
        if randint(0, 99) > 1:
            p += 1
            self['비전수련'] = '%s %d' % (var[0], p) 
            return
        self.attr.__delitem__('비전수련')
        v = self['비전이름']
        if v == '':
            self['비전이름'] = [ var[0] ]
        else:
            if type(v) == str:
                v = [v]
            v.append(var[0])
            self['비전이름'] = v
        self.sendLine('[1m당신이 『[32m%s[37m』의 무공 구결을 깨우치기 시작합니다. \'ΔΨΞλΟ~\'[0;37m\r\n' % var[0])
        self.sendRoom('[1m%s 『[32m%s[37m』의 무공 구결을 깨우치기 시작합니다. \'ΔΨΞλΟ~\'[0;37m' % (self.han_iga(), var[0]))

    def getNextWords(self, line):
        words = line.split(None, 1)
        if len(words) != 2:
            return ''
        return words[1]


for lv in Body.skillLvName:
    lvName = lv + '무공'
    skillList = MAIN_CONFIG[lvName]
    #for name in skillList.split('\n'):
    for name in skillList:
        if name == '':
            continue
        Body.skillLvMap[name.strip()] = Body.skillLv[lv]

