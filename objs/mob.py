# -*- coding: utf-8 -*-

import os
import glob
import time
import copy

from random import randint
from twisted.internet import reactor
from include.define import *
from objs.body import Body
from objs.config import Config, MAIN_CONFIG
from objs.item import Item, is_item, getItem
from objs.script import Script, SCRIPT
from objs.oneitem import Oneitem, ONEITEM
from objs.skill import MUGONG

from lib.loader import load_script, save_script
from lib.func import *
from lib.hangul import *
from objs.droplist import DROPITEM

MAXPROCESSMOVING = 30
REGEN_MULTIPLY = 3

class Mob(Body):

    Mobs = {}
    movingMobs = []
    numMovings = 0
    nMovingOrder = 0

    SkillLevelList = [
        [ '지르기', '비각', '원앙퇴' ],
        [ '쌍비각', '일지관수', '이지관수', '선풍각', '회심퇴', '파옥권', '복호권', '낙영장', '연환원앙퇴', '회중포월', '영오보', '지당편법', '붕박구소'],
        [ '암연소혼장', '뇌격신룡참', '격공진력', '낙화유수', '철사장', '격공장', '복호장', '화선유불장', '삼절연환', '난화불혈수', '벽보신권', '벽공장', '동해신장', '한빙장', '청산장', '혈산설화', '대수인', '혈월비적', '풍마장', '화우폭진', '녹수마장', '혈화비천', '소수겁', '혈화장', '밀종대수인', '현마장', '혈천섬광'],
        [ ],
        [ ],
    ]
    
    def __init__(self):
        self.bPlayer = 0
        self.origin = ''
        self.moveTime = 0
        self.timeofdeath = 0
        self.timeofregen = 0
        self.moveTick = 0
        self.talkTick = 0
        self.moveList = []
        self.skillList = []
        self.skill = None
        self.skillMap = {}
        self.defskillList = []
        self.defskill = None
        self.weapon = ''
        
        self.hp = 0
        self.mp = 0
        
        
        Body.__init__(self)
        
    def create(self, index):
        #print(path)
        self.index = index
        self.path = 'data/mob/' + index.replace(':', '/') + '.json'
        #print(self.path)
        scr = load_script(self.path)
        
        if scr == None:
            return False
        
        try:
            self.attr = scr['몹정보']
        except:
            return False
            
        self.init()
        
    def init(self):
        self.corpse = getInt(self.get('시체'))
        if self.corpse <= 0:
            self.corpse = 30
        else:
            self.corpse = self.corpse * MAIN_CONFIG['REGEN_MULTIPLY']
        self.regen = getInt(self.get('리젠'))
        if self.regen <= 0:
            self.regen = 60
        elif self.regen >= 360:
            self.regen = 360
        else:
            self.regen = self.regen * MAIN_CONFIG['REGEN_MULTIPLY']
            if self.regen >= 360:
                self.regen = 360
        self.setMove()
        
        use_item = self.get('사용아이템')
        if type(use_item) == str:
            use_item_list = [use_item, ] if use_item != "" else []
        else:
            use_item_list = use_item
        for i in use_item_list:
            item = getItem(i.split()[0])
            if item == None:
                continue
            self.armor += getInt(item['방어력'])
            self.attpower += getInt(item['공격력'])
            if item['종류'] == '무기':
                self.weapon = item['전투스크립']
                self.weaponItem = item
        
        l = self['무공']
        for m in l:
            words = m.split()
            if len(words) != 3:
                continue
            s = MUGONG[words[0]]
            if s == None or s == '':
                continue
            if s['종류'] == '전투':
                self.skillList.append( ( s, int(words[1]), int(words[2]) ) )
            else:
                self.defskillList.append( ( s, int(words[1]), int(words[2]) ) )
            
    def reset(self):
        self.target = []
        self.skills = []
        self.dmgMap = {}
        self.dex = 0
        self._str = 0
        self._dex = 0
        self._arm = 0
        self._mp = 0
        self._maxmp = 0
        self._hp = 0
        self._maxhp = 0

        if self['체력'] == '':
            self['체력'] = 0
        if self['내공'] == '':
            self['내공'] = 0
        self.hp = getInt(self.get('체력'))
        self.mp = getInt(self.get('내공'))
        
    def place(self):
        from objs.room import Room, is_room, getRoom
        keydata = self.getString('위치')
        lines = keydata
        lines = keydata if type(keydata) == list else [keydata, ]
        for line in lines:
            for loc in line.split():
                room = getRoom(self.get('존이름') + ':' + loc)
                if room != None:
                    mob = self.clone()
                    mob.reset()
                    mob.origin = self.get('존이름') + ':' + loc
                    room.insert(mob)
                    if len(mob.moveList) != 0:
                        self.movingMobs.append(mob)
    
    def getMp(self):
        if self._mp != 0:
            mp = self.mp + self.mp * self._mp // 100
            return mp
        return self.mp
        
    def getMaxMp(self):
        if getInt(self['내공']) == 0:
            return 0
        if self._maxmp != 0:
            mp = self['내공'] + self['내공'] * self._maxmp // 100
            return float(mp)
        return self['내공']

    def getHp(self):
        return self.hp
        
    def getMaxHp(self):
        if getInt(self['체력']) == 0:
            return 0
        if self._maxhp != 0:
            hp = self['체력'] + self['체력'] * self._maxhp // 100
            return float(hp)
        return self['체력']

    def addItem(self):
        if len(self.objs) != 0:
            return
        d = self['난이도']
        if d == '':
            d = 0

        iList = self['아이템']
        for i in iList:
            c = 1
            words = i.split()
            if len(words) < 3:
                continue
            if len(words) == 4:
                c = getInt(words[3])
            index = words[0]
            item = getItem(index)
            if item == None:
                continue
            if item.isOneItem() and item.isOneThere():
                continue
            count = getInt(words[1])
            chance = getInt(words[2])
            if d > 0:
               chance = int( chance * Body.difficulty[d-1][2] ) 
            for cnt in range(count):
                if chance >= randint(0, 100 * c):
                    obj = item.deepclone()
                    obj.applyMagic(self['레벨'], 0)
                    self.insert(obj)
                    
        iList = self['사용아이템']
        for i in iList:
            c = 1
            words = i.split()
            if len(words) < 3:
                continue
            if len(words) == 4:
                c = getInt(words[3])
            index = words[0]
            item = getItem(index)
            if item == None:
                continue
            if item.isOneItem() and item.isOneThere():
                continue
            count = getInt(words[1])
            chance = getInt(words[2])
            for cnt in range(count):
                if chance >= randint(0, 100 * c):
                    obj = item.deepclone()
                    obj.applyMagic(self['레벨'], 0)
                    self.insert(obj)
        
    def viewItemList(self):
        if len(self.objs) == 0:
            return '[36m☞ 아무것도 없습니다.[37m'
        msg = ''
        for obj in self.objs:
            msg += '[36m%s[37m\r\n' % obj['이름']
        return msg[:-2]
        
    def view(self, ob):
        if self.act == ACT_DEATH:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('[0m[44m[1m[37m◆ 이름 ▷ %-49s[0m[37m[40m' % (self.get('이름') + '의 시체'))
            ob.sendLine('──────────────────────────────')
            ob.sendLine(self.viewItemList())
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            return
            
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        ob.sendLine('[0m[44m[1m[37m◆ 이름 ▷ %-49s[0m[37m[40m' % self.get('이름'))
        ob.sendLine('──────────────────────────────')
        desc = self['설명2']
        if type(desc) == list:
            ob.sendLine('\r\n'.join(desc))
        else:
            ob.sendLine(desc)
        ob.sendLine('──────────────────────────────')
        
        l = self.get('사용아이템')
        for lv in self.ItemLevelList:
            for i in l:
                item = getItem(i.split()[0])
                if item == None:
                    continue
                if lv == item['계층']:
                    ob.sendLine('[%s] [36m%s[37m' % (self.ItemUseLevel[item.get('계층')] , item.get('이름')))
        if len(l) != 0:
            ob.sendLine('──────────────────────────────')
        ob.sendLine('★ %s' % self.GetHPString())
        ob.sendLine('☆ %s' % self.getHPbar())
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

    def checkDieEvent(self):
        e1 = self.get('이벤트 $%소멸이벤트%')
        e2 = self.get('이벤트: $%소멸이벤트%')
        
        if e1 != '':
            return '이벤트 $%소멸이벤트%'
        elif e2 != '':
            return '이벤트: $%소멸이벤트%'
        
        return ''
            
    def setMove(self):
        rstr = self.get('이동')
        if rstr == '':
            return
        if type(rstr) == list:
            rstr = " ".join(rstr)
        self.moveTick = getInt(self.get('이동틱'))
        if self.moveTick == 0:
            self.moveTick = 30
        
        mr = rstr.split()
        for r in mr:
            if r.find('-') != -1:
                rs = r.split('-')
                if len(rs) != 2:
                    continue
                for n in range( int(rs[0]), int(rs[1]) ):
                    rName = self.get('존이름') + ':' + str(n)
                    if rName not in self.moveList:
                        self.moveList.append(rName)
            else:
                rName = self.get('존이름') + ':' + r
                if rName not in self.moveList:
                    self.moveList.append(rName)
        
    def updateMoving(self):
        if self.numMovings == 0:
            return
        #print 'updateMovings'
        for n in range(MAXPROCESSMOVING):
            if  self.nMovingOrder >= self.numMovings:
                self.nMovingOrder = 0
                break
            
            mob = self.movingMobs[self.nMovingOrder]
            if mob != None:
                mob.move()
                #print str(self.nMovingOrder) + ': ' + mob.get('이름')
            self.nMovingOrder += 1
            
        if self.nMovingOrder >= self.numMovings:
            self.nMovingOrder = 0
        
    def move(self):
        
        if self.env == None:
            return
        if self.act != ACT_STAND:
            return
            
        if self.moveTime == 0:
            self.moveTime = time.time()
        curTime = time.time()
        if curTime < self.moveTime + self.moveTick:
            return
        if randint(0, 2) != 0:
            return
        room, dir = self.env.getRandomExit()
        if dir not in ['동', '서', '남', '북', '위', '아래', '북동', '북서', '남동', '남서']:
            return
        if room == None:
            return
            
        if room.index in self.moveList:
            #print str(self.nMovingOrder) + ': ' + self.get('이름') + ' ' + room.index
            self.enterRoom(room, dir)
    
    def enterRoom(self, room, dir):
        self.moveTime = time.time()
        #print self.get('이름') + ' ' + room.index + '/' + dir
        
        msg1 = self.get('퇴진스크립')
        if msg1 == '':
            msg1 = '$방향$쪽으로 갔습니다.'
        msg1 = msg1.replace('$방향$', dir)
        msg2 =  self.get('진입스크립')
        if msg2 == '':
            msg2 = '$방향$쪽에서 왔습니다.'
        msg2 = msg2.replace('$방향$', room.reverseDir[dir])
        self.env.sendRoom('\r\n[33m' + self.get('이름') + '[37m' + han_iga(self.get('이름')) + ' ' + msg1)
        self.env.remove(self)
        msg = '\r\n[33m' + self.get('이름') + '[37m' + han_iga(self.get('이름')) + ' ' + msg2
        say = self.getSayStr()
        if say != '' and randint(0,2) == 0:
            msg += '\r\n' + say
        room.sendRoom(msg)
        room.insert(self)
        
    def getNameA(self):
        return '[33m' + self.get('이름') + '[37m'
        
    def say(self):
        say = self.getSayStr()
        if say != '':
            self.env.writeRoom('\r\n' + say)
                    
    def update(self):
        self.tick += 1
        curTime = time.time()
        if self.tick % 60 == 0:
            self.recover()
        
        if self.act == ACT_STAND:
            if self.get('대화틱') != '' and self.tick % self.get('대화틱') == 0:
                if randint(0, 2) == 0:
                    self.say()
                    return True
        elif self.act == ACT_DEATH:
            if curTime - self.timeofdeath >= self.corpse + self.regen:
                self.doDeath(curTime - self.timeofdeath - self.corpse)
                self.doRegen()
                return True
            elif curTime - self.timeofdeath >= self.corpse:
                self.doDeath()
                return True
        elif self.act == ACT_REGEN:
            if curTime - self.timeofdeath >= self.corpse + self.regen:
                self.doRegen()
                return True
        elif self.act == ACT_REST:
            if curTime - self.timeofdeath >= self.regen:
                self.doRegen()
                return True
        if self['몹종류'] == 6:
            r = self['아이템리젠']
            if r == '':
                r = 180
            if r < 180:
                r = 180
            if curTime - self.timeofregen >= r:
                self.timeofregen = curTime
                self.addItem()
        elif self['전투종류'] == 1 and self.act == ACT_STAND:
            from objs.player import Player, is_player
            for ply in self.env.objs:
                if is_player(ply) and ply['투명상태'] != 1:
                    ply.setFight(self, True)  
                    break
            
        if self.checkDefenceSkill(curTime):
            return True
        return False
        
    def recover(self):
        #체력회복
        hp = self.hp
        maxhp = self.get('체력')
        
        mp = self.getMp()
        maxmp = self.getMaxMp()
        
        if self.act == ACT_STAND:
            # 10% 회복
            r = 0.1
        elif self.act == ACT_REST:
            # 20% 회복
            r = 0.2
        elif self.act == ACT_FIGHT:
            # 5% 회복
            r = 0.05
        else:
            return

        if hp < maxhp:
            hp += int (maxhp * r)
            if hp >= maxhp:
                hp = maxhp
            self.hp = hp
        
        if mp < maxmp:
            mp += int (maxmp * r)
            if mp >= maxmp:
                mp = maxmp
            self.mp = mp

    def checkDefenceSkill(self, curTime, stop = False):
        skills = copy.copy(self.skills)
        msg = ''
        for s in skills:
            if stop == True:
                s.end_time = 0
            if s.end_time < curTime:
                self.skills.remove(s)
                buf1, buf2, buf3 = self.makeFightScript(s['무공해제스크립'], None)
                self._str -= s._str
                self._dex -= s._dex
                self._arm -= s._arm
                self._mp -= s._mp
                self._maxmp -= s._maxmp
                self._hp -= s._hp
                self._maxhp -= s._maxhp

                msg += '\r\n' + buf3
                #print msg
                del s
        if len(msg) != 0:
            self.env.writeRoom(msg)
            return True
        return False
                
    def getExpGold(self, target):
        c1 = getInt(target['레벨'])
        c2 = getInt(self['레벨'])
        a=((c2*c2)//3)+30
        b=(a * (c2-c1))//100
    
        c = a + b
        #print c1, c2, a, b, c
        if c < 1:
            c = 1;
        if c > MAX_INT:
            c = MAX_INT
        c2 = randint(0, 9)
        if randint(0, 1) == 0:
            c += c2
        else:
            c -= c2;
        if c < 1:
            c = 1
        if c > MAX_INT:
            c = MAX_INT
        
        c1 = getInt(self['레벨']) + 14
        c2 = randint(0, 4)
        if randint(0, 1) == 0:
            c1 += c2;
        else:
            c1 -= c2;
        #print self['은전']
        c1 += getInt(self['은전'])
        if c1 < 1:
            c1 = 1
        if c1 > MAX_INT:
            c1 = MAX_INT
        
        return c, c1
        
    def addHerb(self):
        if len(self.target) == 0:
            return
        if self['레벨'] < self.target[0]['레벨']:
            return
        p1 = self['레벨'] - self.target[0]['레벨']
        p2 = p1 * 0.01 + 0.05
        try:
            d = float (self['난이도']) 
            p2 += d
        except:
            pass
        p3 = randint(0, 99)
        
        if p2 > MAIN_CONFIG['약초나올확률']:
            p2 = MAIN_CONFIG['약초나올확률']
        if p2 < p3:
            return
        
        herbs = MAIN_CONFIG['내공아이템리스트']
        l = len(herbs)
        herb = getItem(herbs[randint(0, l - 1)]).clone()
        if len(self.target) != 0:
            self.target[0].insert(herb)
            
    def die(self, killer):
        from objs.player import Player, is_player
        self._str = 0
        self._dex = 0
        self._arm = 0
        self.addItem()
        self.addHerb()
        msg = self.get('소멸스크립')
        if msg == '':
            self.env.writeRoom('\r\n[1;37m' + self.getName() + han_iga(self.getName()) + ' 쓰러집니다. \'쿠웅~~ 철퍼덕~~\'[0;37m')
        else:
            self.env.writeRoom('\r\n[1;37m' + msg + '[0;37m')
        self.env.writeRoom('\r\n')
        #print len(self.target)
        c = 0
        n = 0
        lv = 0
        maxLvTarget = None

        for target in self.target:
            if self.env == target.env:
                n += 1
                if target['레벨'] > lv:
                    lv = target['레벨']
                    maxLvTarget = target

        for target in self.target:
            if self.env != target.env:
                continue
            who = target['이름']
            if who not in self.dmgMap:
                continue

            dmg = float(self.dmgMap[who])
            ratio = dmg // float(self['체력'])
            c += 1
            if ratio > 1:
                ratio = 1
            #print dmg, self['체력'], ratio
            exp, gold = self.getExpGold(maxLvTarget)
            exp = int( exp // n )
            gold = int ( gold // n )

            """
            exp = exp // 2
            gold = gold // 2
            exp = int( exp * ratio ) + int( exp // n)
            gold = int ( gold * ratio ) + int (gold // n)
            """
            bonus_exp = 0
            bonus_gold = 0
            
            #if is_player(target) and target['레벨'] > self['레벨']:
            #    target.addStr(5);
            #    target.addDex(2);
            #    target.weaponSkillUp(3);
            try:
                d = int(self['난이도'])
            except:
                d = 0
            if d != 0:
                bonus_exp = int( exp * Body.difficulty[d-1][2] ) 
                bonus_gold = int( gold * Body.difficulty[d-1][3] ) 
                target.sendLine('\r\n당신이 %d(+%d)의 경험치를 얻습니다.' % (exp, bonus_exp))
                target.sendLine('당신이 %s에게 은전 %d(+%d)개를 획득합니다.' % (self.getNameA(), gold, bonus_gold))
            else:
                target.sendLine('\r\n당신이 %d의 경험치를 얻습니다.' % exp)
                target.sendLine('당신이 %s에게 은전 %d개를 획득합니다.' % (self.getNameA(), gold))
            target['은전'] += gold + bonus_gold
            target['%d 성격플킬' % getInt(self['성격'])] += 1
            
            msg = '%s 약간의 경험치를 얻습니다.\r\n' % target.han_iga()
            msg += '%s 몇개의 은전을 획득합니다.' % target.han_iga()
            
            if c == 1 and target.checkConfig('자동습득') == True:
                chance = randint(0, 99)
                if self['레벨'] >= 2000 and chance < 1:
                    dropitem = DROPITEM[randint(0, len(DROPITEM) - 1)]
                    item = getItem(dropitem)
                    if item != None:
                        if target.getItemCount() <= getInt(MAIN_CONFIG['사용자아이템갯수']) and target.getItemWeight() + item['무게'] < target.getStr() * 10:
                            obj = item.deepclone()
                            if randint(0, 99) < 30:
                                obj.applyMagic(self['레벨'], 0)
                            target.insert(obj)
                            target.sendLine('당신이 %s 전리품으로 획득합니다.' % item.han_obj())
                            msg += '\r\n%s %s 전리품으로 획득합니다.' % (target.han_iga(), item.han_obj())
                            
                objs = copy.copy(self.objs)
                for item in objs:
                    if target.getItemCount() > getInt(MAIN_CONFIG['사용자아이템갯수']) or target.getItemWeight() + item['무게'] > target.getStr() * 10:
                        break
                    self.remove(item)
                    target.insert(item)
                    if item.isOneItem():
                        ONEITEM.have(item.index,target['이름'])
                    target.sendLine('당신이 %s 전리품으로 획득합니다.' % item.han_obj())
                    msg += '\r\n%s %s 전리품으로 획득합니다.' % (target.han_iga(), item.han_obj())
            target.sendRoom(msg, noPrompt = True)
            
            target.addExp(exp + bonus_exp)
        
        dieEvent = self.checkDieEvent()
        if dieEvent != '':
            reactor.callLater(0, target.doEvent, self, dieEvent, '')
        if len(self.target) != 0:
            target.env.printPrompt(killer, False)
        self.act = ACT_DEATH
        self.clearTarget()
        self.clearSkills()
        self.timeofdeath = time.time()
        
    def minusHP(self, demage, mode = True, who = ''):
        if demage > self.hp:
            demage = self.hp
        if who not in self.dmgMap:
            self.dmgMap[who] = demage
        else:
            self.dmgMap[who] += demage
        self.hp -= demage
        if self.hp <= 0:
            self.die(who)
            return True
        return False
        
    def doDeath(self, sec = None):
        self.act = ACT_REGEN
        self.env.writeRoom('\r\n' + self.getNameA() + '의 시체가 무림지존의 손에 이끌려 망자의 강을 건너갑니다.')
        if len(self.objs) > 0:
            objs = copy.copy(self.objs)
            msg = '\r\n'
            for obj in objs:
                msg += '%s의 시체속에서 %s 모습을 드러냅니다.\r\n' % (self.getNameA(), obj.han_iga())
                self.remove(obj)
                self.env.insert(obj)
                obj.drop(sec)
            self.env.writeRoom(msg[:-2])
        
    def doRegen(self):
        self._str = 0
        self._dex = 0
        self._arm = 0
        from objs.room import getRoom
        self.reset()
        self.act = ACT_STAND
        #원래 index로 복귀하라!
        if self.origin != self.env.index:
            self.env.remove(self)
            room = getRoom(self.origin)
            room.insert(self)
        self.env.writeRoom('\r\n' + self.get('설명3'))
        self.attack_player()
        
    def attack_player(self):
        from objs.player import Player, is_player
        # 선공몹일 경우 플레이어 공격
        if self.get('전투종류') == 1:
            #print '리젠선공몹2!!'
            for p in self.env.objs:
                if is_player(p) and p['투명상태'] != 1:
                    #print '리젠선공몹3!!'
                    p.setFight(self, True)
                    break
        
    def getSayStr(self):
        lines = self.get('자동스크립')
        if len(lines) == 0:
            return ''
        return lines[randint(0, len(lines) - 1)]
        
    def getDesc1(self):
        msg = ''
        for s in self.skills:
            msg += s['방어상태머리말'] + ' '
        return msg + self.get('설명1')
    
    def checkEvent(self, words):
        noissue = ''
        for key in self.attr:
            if key.find('이벤트') == 0:
                keywords = key[4:].split()
                cmdList = []
                issueList = []
                for keyword in keywords:
                    if keyword[0] == '$':
                        cmdList.append(keyword[1:])
                    else:
                        issueList.append(keyword)
                    
                if words[-1] in cmdList:
                    #print self.attr[key]
                    if len(issueList) == 0:
                        noissue = key
                    if len(words) > 2 and words[-2] not in issueList:
                        continue
                    elif len(words) == 2 and len(issueList) != 0:
                        continue
                    #self.doEvent(player, key, words)
                    return key
        if noissue != '':
            #self.doEvent(player, noissue, words)
            return noissue
        return ''
    
    def getFightStartStr(self):
        return '[33m' + self.get('이름') + '[37m' + han_iga(self.get('이름')) + ' ' + self.get('전투시작'), ''
        
    def getHPbar(self):
        maxhp = self.get('체력')
        hcnt = 10*self.hp//maxhp
        return self.strBar[hcnt] + ' (%d)' % (100 * self.hp // maxhp)
        
    def get_hp_script(self):
        maxhp = self.get('체력')
        cnt = len(self.hp_script)
        s = self.hp_script[(cnt - 1) - ((cnt - 1) * self.hp // maxhp)]
        s = self['이름'] + postPosition(s, self['이름'])
        return s
        
    def GetHPString(self):
        mode = self['체력스크립']
        if mode == '':
            mode == '사람'
        mode += '스크립'
        scripts = SCRIPT[mode]
        cnt = len(scripts)
        if cnt == 0:
            return ''
        ix = (cnt - 1) - ((cnt - 1) * self.hp // self['체력'] )
        if ix < 0:
            ix = 0
        if ix >= cnt:
            ix = cnt - 1
        s = scripts[ix]
        s = self['이름'] + postPosition(s, self['이름'])
        return s

    def getWeapon(self):
        if self.weaponItem != None:
            return self.weaponItem
        return getItem('주먹')
        
    def getAttackFailScript(self, mob):
        if self.weapon == '':
            buf = self['전투스크립']
        else:
            buf = self.weapon
            
        s = SCRIPT[buf + '전투실패스크립']
        s = s[randint(0, len(s) - 1)]
        
        
        return self.makeFightScript(s, mob)
        
    def getAttackScript(self, mob, dmg, c1, c2):
        if self.weapon == '':
            buf = self['전투스크립']
        else:
            buf = self.weapon
        s = SCRIPT[buf + '전투스크립']
        c = ((dmg - c1) * (len(s) - 1))//(c2-c1)
        #print dmg, c1, c2, c, len(s)
        i = len(s) - 1 - c
        if i < 0 or i > len(s) - 1:
            print ('mob.getAttackScript')
            i = 0
        s = s[i]
        #s = s[randint(0, len(s) - 1)]

        return self.makeFightScript(s, mob)
        
    def getAct(self):
        if self.act == ACT_STAND:
            return '보통'
        elif self.act == ACT_REST:
            return '쉼'
        elif self.act == ACT_FIGHT:
            return '전투'
        elif self.act == ACT_DEATH:
            return '시체'
        elif self.act == ACT_REGEN:
            return '리젠'
            
    def setAct(self, act):
        if act == '보통':
            self.act = ACT_STAND
        elif act == '쉼':
            self.act = ACT_REST
        elif act == '전투':
            self.act = ACT_FIGHT
        elif act == '시체':
            self.act = ACT_DEATH
            self.timeofdeath = time.time()
        elif act == '리젠':
            self.doDeath()
        elif act == '리젠후생성':
            self.doRegen()
            
    def setSkill(self):
        if self.skill != None:
            return False
        for skill in self.skillList:
            if skill[0]['종류'] != '전투':
                continue
            if self.hp > self['체력'] * skill[1] // 100:
                continue
            if skill[2] < randint(0, 100):
                continue
            if skill[0].mp > self.getMp():
                continue
            if self.lastskill != None and self.lastskill.name == skill[0].name:
                self.skill = self.lastskill
            else:
                self.skill = copy.copy(skill[0])
            self.skill.init()
            self.mp -= self.skill.mp
            return True
        return False

    def setAttackSkill(self):
        if self.skill != None:
            return False
        if len(self.skillList) !=0:
            c = randint(0, len(self.skillList)-1)
            skill = self.skillList[c]
            if skill[0]['종류'] != '전투':
                return False
            if self.hp > self['체력'] * skill[1] // 100:
                return False
            if skill[2] < randint(0, 100):
                return False
            if skill[0].mp > self.getMp():
                return False
            if self.lastskill != None and self.lastskill.name == skill[0].name:
                self.skill = self.lastskill
            else:
                self.skill = copy.copy(skill[0])
            self.skill.init()
            self.mp -= self.skill.mp
            return True
        return False

    def checAntiSkill(self, skill, target):
        for ss in target.skills:
            if skill['이름'] == ss.name or skill['계열'] == ss.getAntiType():
                return True
        return False
    
    def setDefenceSkill(self, ob):
        if len(self.skillList) ==0:
            return False
        for skill in self.defskillList:
            sk = skill[0]
            if self.hp > self['체력'] * skill[1] // 100:
                continue
            if skill[2] < randint(0, 100):
                continue
            if sk.mp > self.getMp():
                continue
            attr = sk['속성']
            if self.checAntiSkill(sk, ob):
                continue
            if self.checAntiSkill(sk, self):
                continue
            s = copy.copy(sk)
            self.skillUp(s)
            t = self.skillMap[s.name][0]
            against = ''
            plus = 0
            for at in attr:
                if at.find('상대무공') == 0:
                    aName = at[9:]
                    against = MUGONG[aName].clone()
                    break
            if against != '':
                #chance = self.getAttackChance(ob)
                if s['계열'] == '내공흡수' and ob.getMp() > 0:
                    try:
                        plus = ob.mp * against._mp // 100 * -1
                        if plus + self['내공'] > self['최고내공']:
                            plus = self['최고내공'] - self['내공']
                        if plus < 0:
                            plus = 0
                        self['내공'] += plus
                        ob.mp -= plus
                    except:
                        plus = ob['내공'] * against._mp // 100 * -1
                        if plus + self['내공'] > self['최고내공']:
                            plus = self['최고내공'] - self['내공']
                        if plus < 0:
                            plus = 0
                        self['내공'] += plus
                        ob['내공'] -= plus
                elif s['계열'] == '내공감소':    
                    ob._mp += against._mp
                    ob._maxmp += against._maxmp
                    ob.skills.append(against)
                    against.end_time = time.time() + against['방어시간'] + against['방어시간증가치'] * (t - 1)
                    #against.start_time = against['방어시간'] + against['방어시간증가치'] * (t - 1)
                elif s['계열'] == '체력흡수' and ob.getHp() > 0: #체력흡수 추가 
                    try:
                        plus = ob.hp * against._hp // 100 * -1
                        if plus + self['체력'] > self['최고체력']:
                            plus = self['최고체력'] - self['체력']
                        if plus < 0:
                            plus = 0
                        self['체력'] += plus
                        ob.hp -= plus
                    except:
                        plus = ob['체력'] * against._hp // 100 * -1
                        if plus + self['체력'] > self['최고체력']:
                            plus = self['최고체력'] - self['체력']
                        if plus < 0:
                            plus = 0
                        self['체력'] += plus
                        ob['체력'] -= plus
                elif s['계열'] == '체력감소':    
                    ob._hp += against._hp
                    ob._maxhp += against._maxhp
                    ob.skills.append(against)
                    against.end_time = time.time() + against['방어시간'] + against['방어시간증가치'] * (t - 1)
                    #against.start_time = against['방어시간'] + against['방어시간증가치'] * (t - 1)
                self.skills.append(s)
            else:
                self.skills.append(s)
            s.end_time = time.time() + s['방어시간'] + s['방어시간증가치'] * (t - 1)
            #s.start_time = s['방어시간'] + s['방어시간증가치'] * (t - 1)
            self._str += s._str
            self._dex += s._dex
            self._arm += s._arm
            self._mp += s._mp
            self._maxmp += s._maxmp
            #print(s.end_time, s.start_time)
            buf1, buf2, buf3 = self.makeFightScript(s['무공스크립'], ob)
            buf11, buf12, buf13 = self.makeFightScript(s['실패'], ob)
            try:
                if plus > 0:
                    ob.sendLine(buf3 + ' ([1;31m+ %d[0;37m)' % plus)
                else:
                    ob.sendLine(buf3)
            except:
                ob.sendLine(buf3)
            ob.sendFightScriptRoom(buf3, ex = ob)
            return True
        return False

    def getSkillChance(self, mob):
        l1 = self['레벨']
        l2 = mob['레벨']
        if self.skill != None:
            CHANCE = self.skill['확률']
        else:
            CHANCE = 100
        bonus = getInt(self['명중']) * float(MAIN_CONFIG['명중확률'])
        bonus -= getInt(mob['회피']) * float(MAIN_CONFIG['회피확률'])
        return CHANCE - (((l2-l1)+90)//3) + bonus

#난이도수정
    def parseDiffconfig(self, config, value):
        diffconfig={}
        for obj in config:
            para = obj.split(',')
            key = para[0].strip()
            diffconfig[key] = {}
            for cnt in range(0,value):
                diffconfig[key]['값%d' % (cnt+1)] = para[cnt+1]
        return diffconfig

    def getValuePara(self, para):
        var = float(para['값1'])
        min = float(para['값2'])
        max = float(para['값3'])
        rate = float(para['값4'])
        randmin = float(para['값5'])//100.0
        randmax = float(para['값6'])//100.0
        return var, rate, min, max, randmin, randmax

    def checkMinMax(self, var, min, max):
        if var < min:
            var = min
        elif var > max:
            var = max
        return var

    def setDifficulty(self):
        try:
            d = int(self['난이도'])
        except:
            d = 0
        if d == 0:
            return
        d -= 1
        #l = self['레벨'] + 2000 * (d + 1) - 500
        #self['레벨'] = l
        #self['체력'] = int( self['체력'] * self.difficulty[d][0] + 200000 * d)
        #hp = int( 0.0529 * l * l - 8.7552 * l + 2448.9 )
        #self['체력'] = int( hp * self.difficulty[d][2])
        #self['힘'] = int( self['힘'] * self.difficulty[d][1] + 5500 * d )

        #self['힘'] = int( l * (3.0 + l // 20000)  * 1.3 )
        plv = d + 1
        para = MAIN_CONFIG[('난이도파라미터%d' % plv )]
        diffpara={}
        diffpara = self.parseDiffconfig(para, 2)
        startlv = getInt(diffpara['시작레벨']['값1'])
        maxlv = getInt(diffpara['시작레벨']['값2'])
        hpinc1 = float(diffpara['체력기울기']['값1'])
        hpinc2 = float(diffpara['체력기울기']['값2'])
        strinc1 = float(diffpara['힘기울기']['값1'])
        strinc2 = float(diffpara['힘기울기']['값2'])

        l = self['레벨'] + startlv * plv
        self['레벨'] = l

        config = MAIN_CONFIG[('난이도설정값%d' % plv )]
        diffconfig = {}
        diffconfig = self.parseDiffconfig(config, 6)

        hpvar, hprate, hpmin, hpmax, hprandmin, hprandmax = self.getValuePara(diffconfig['체력'])
        hpvar = hpvar + randint(int(-hpvar * hprandmin), int(hpvar * hprandmax))
        hp = int( hpinc1 * l * l - hpinc2 * l + hpvar )
        self['체력'] = int(self.checkMinMax((hp * hprate), hpmin, hpmax))

        strvar, strrate, strmin, strmax, strrandmin, strrandmax = self.getValuePara(diffconfig['힘'])
        strvar = strvar + randint(int(-strvar * strrandmin), int(strvar * strrandmax))
        str = int( l * (strinc1 + l // strrate)  * strinc2  + strvar)
        self['힘'] = int(self.checkMinMax(str, strmin, strmax))

        armvar, armrate, armmin, armmax, armrandmin, armrandmax = self.getValuePara(diffconfig['맷집'])
        armvar = armvar + randint(int(-armvar * armrandmin), int(armvar * armrandmax))
        arm = int(l * armrate + armvar)
        if self['맷집'] >= armmax:
            self['맷집'] = self['맷집'] + arm
        else:
            self['맷집'] = int(self.checkMinMax(arm, armmin, armmax))

        dexvar, dexrate, dexmin, dexmax, dexrandmin, dexrandmax = self.getValuePara(diffconfig['민첩성'])
        dexvar = dexvar + randint(int(-dexvar * dexrandmin), int(dexvar * dexrandmax))
        dex = int(self['레벨'] * dexrate) + dexvar
        self['민첩성'] = int(self.checkMinMax(dex, dexmin, dexmax))

        mpvar, mprate, mpmin, mpmax, mprandmin, mprandmax = self.getValuePara(diffconfig['내공'])
        mpvar = mpvar + randint(int(-mpvar * mprandmin), int(mpvar * mprandmax))
        mp = int(l // mprate + mpvar)
        self['내공'] = int(self.checkMinMax(mp, mpmin, mpmax))

        hitvar, hitrate, hitmin, hitmax, hitrandmin, hitrandmax = self.getValuePara(diffconfig['명중'])
        self['명중'] = randint( hitmin, hitmax)

        missvar, missrate, missmin, missmax, missrandmin, missrandmax = self.getValuePara(diffconfig['회피'])
        self['회피'] = randint( missmin, missmax)

        crivar, crirate, crimin, crimax, crirandmin, crirandmax = self.getValuePara(diffconfig['필살'])
        self['필살'] = randint( crimin, crimax)

        luckvar, luckrate, luckmin, luckmax, luckrandmin, luckrandmax = self.getValuePara(diffconfig['운'])
        self['운'] = randint( luckmin, luckmax)

        if self['무공'] == '':
            h1 = 80
            c1 = 70
            h = h1 + self['레벨'] * (100-h1) // maxlv
            if h > 100:
                h = 100
            c = c1 + self['레벨'] * (100-c1) // maxlv
            if c > 100:
                c = 100
            s = MUGONG['독%d' % (d + 1)]
            self.skillList.append( ( s, h, c ) )

        self.hp = self['체력']
        self.mp = self['내공']
        self['보너스'] = self.difficulty[d][2]

        self.skillList = []
        dd = d
        if dd > 3:
            dd = 3 
        sl = self.SkillLevelList[dd] + self.SkillLevelList[dd + 1]
        s = MUGONG[ sl[randint(0, len(sl) - 1)] ]
        if s != None and s != '':
            self.skillList.append( ( s, 100, 100) )

def is_mob(obj):
    return isinstance(obj, Mob)

def getMob(path):

    i = path.find(':')
    if i == -1:
        return None

    zoneName = path[:i]
    mobName = path[i+1:]

    try:
        zone = Mob.Mobs[zoneName]
    except KeyError:
        zone = {}
        Mob.Mobs[zoneName] = zone
        
    try:
        mob = zone[mobName]
    except KeyError:
        mob = Mob()
        ret = mob.create(path)
        if ret == False:
            return None

        zone[mobName] = mob

    return mob

def loadAllMob():
    log('몹 로딩중... 잠시만 기다려주세요.')
    pwd = os.getcwd()
    c = 0
    curTime = time.time()
    dirs = os.listdir('data/mob')
    for dir in dirs:
        try:
            os.chdir('data/mob/' + dir)
        except:
            continue
        files = glob.glob('*.json')
        #print files
        os.chdir(pwd)
        for file in files:
            mob = getMob(dir + ':' + file[:-5])
            if mob != None:
                mob['존이름'] = dir
                if dir[-1].isdigit():
                    mob['난이도'] = int(dir[-1])
                mob.setDifficulty()
                c = c + 1
                mob.place()
                mob.timeofregen = curTime
    log(str(c) + '개의 몹이 로딩되었습니다.')
    Mob.numMovings = len(Mob.movingMobs)
    log(str( Mob.numMovings ) + '개의 활동 몹이 로딩되었습니다.')
    

