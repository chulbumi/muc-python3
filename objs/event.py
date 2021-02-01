# -*- coding: utf-8 -*-

from lib.hangul import postPosition1, han_iga, han_obj
from objs.rank import RANK
from objs.oneitem import ONEITEM
from objs.skill import MUGONG
from objs.room import getRoom
from lib.func import getStrCnt, getInt, stripANSI
from lib.func import loadScriptFile
from include.define import *
import copy
import time

def doEvent(self, mob, key, words, lineNumber = None):
    
    from lib.hangul import postPosition1, han_iga, han_obj
    from objs.rank import RANK
    from objs.oneitem import ONEITEM
    from objs.skill import MUGONG
    from objs.room import getRoom
    from lib.func import getStrCnt, getInt, stripANSI
    from lib.func import loadScriptFile
    import copy
    import time

    lineNum = 0
    tab = 0
    rank1 = 0
    rank2 = 0
    searchEnd = False
    broadcast = False
    sub = {}
    script = mob.get(key)
    for line in script:
        lineNum += 1
        if lineNumber != None:
            if lineNum <= lineNumber:
                continue

        if line.strip() == '':
            if searchEnd == False:
                self.sendLine('')
            continue
        sline = line.strip()

        if searchEnd:
            if sline[0] == '{':
                tab = tab + 1
                continue
            elif sline[0] != '}':
                continue
            tab = tab - 1
            if tab != 0:
                continue
            searchEnd = False
            tab = 0
            continue

        if sline.find('$종료') == 0:
            break

        sline = sline.replace('[사용자이름]', self['이름'])

        if sline[0] == '$':
            if len(words) > 2:
                sline = sline.replace('$변수:1', words[1])

            func = line.split()[0]
            nextWords = self.getNextWords(sline).strip()

            #self.sendLine( line )
            if func == '$엔터$':
                self._eventTemp = (mob, key, words, lineNum)
                self.sendLine(nextWords)
                self.input_to(self.pressEnter2)
                self.INTERACTIVE = 0
                return
            elif func == '$입력대기출력끝$':
                self.input_to(self.parse_command)
                self.INTERACTIVE = 1
            elif func == '$이벤트확인!':
                if self.checkEvent(nextWords) == True:
                    searchEnd = True
            elif func == '$이벤트확인':
                if self.checkEvent(nextWords) == False:
                    searchEnd = True
            elif func == '$이벤트설정':
                self.setEvent(nextWords)
            elif func == '$이벤트삭제':
                self.delEvent(nextWords)
            elif func == '$착용확인!':
                index, cnt = getStrCnt(sline)
                if '$변수:' in line:
                    item = self.getItemName(index)
                    if item != None and item.inUse == False:
                        searchEnd = True
            elif func == '$아이템속성확인!':
                index, cnt = getStrCnt(sline)
                if '$변수:' in line:
                    item = self.getItemName(index)
                    if item != None and item.getOption() != None:
                        searchEnd = True
            elif func == '$강화확인2000!':
                index, cnt = getStrCnt(sline)
                item = self.getItemIndex(index)
                print('11')
                if item != None and item['공격력'] < 2000:
                    searchEnd = True
                    print('22')
            elif func == '$강화확인4000!':
                index, cnt = getStrCnt(sline)
                item = self.getItemIndex(index)
                print('11')
                if item != None and item['공격력'] < 4000:
                    searchEnd = True
                    print('22')
            elif func == '$아이템확인!':
                index, cnt = getStrCnt(sline)
                if index == '은전' and cnt < 1:
                    continue
                if index == '금전' and cnt < 1:
                    continue
                if '$변수:' in line:
                    if self.checkItemName(index, cnt) == True:
                        searchEnd = True
                else:
                    if self.checkItemIndex(index, cnt) == True:
                        searchEnd = True
            elif func == '$아이템착용확인!':
                index, cnt = getStrCnt(sline)
                if index == '은전' and cnt < 1:
                    continue
                if index == '금전' and cnt < 1:
                    continue
                if '$변수:' in line:
                    if self.checkItemName(index, cnt, True) == True:
                        searchEnd = True
                else:
                    if self.checkItemIndex(index, cnt, True) == True:
                        searchEnd = True
            elif func == '$속성설정':
                if nextWords != '':
                    self[nextWords] = 1
            elif func == '$아이템옵션삭제':
                index, cnt = getStrCnt(sline)
                if '$변수:' in line:
                    item = self.getItemName(index)
                    if item != None:
                        item.delOption()
                    else:
                        searchEnd = True
            elif func == '$아이템확인':
                index, cnt = getStrCnt(sline)
                if index == '은전' and cnt < 1:
                    searchEnd = True
                    continue
                if index == '금전' and cnt < 1:
                    searchEnd = True
                    continue
                if '$변수:' in sline:
                    if self.checkItemName(words[1], cnt) == False:
                        searchEnd = True
                else:
                    if self.checkItemIndex(index, cnt) == False:
                        searchEnd = True
            elif func == '$아이템착용확인':
                index, cnt = getStrCnt(sline)
                if index == '은전' and cnt < 1:
                    searchEnd = True
                    continue
                if index == '금전' and cnt < 1:
                    searchEnd = True
                    continue
                if '$변수:' in sline:
                    if self.checkItemName(words[1], cnt, True) == False:
                        searchEnd = True
                else:
                    if self.checkItemIndex(index, cnt, True) == False:
                        searchEnd = True
            elif func == '$아이템확장확인':
                item = self.getItemName(words[1])
                if item != None and item.get('확장 이름') != '':
                    continue
                else:
                    searchEnd = True
            elif func == '$아이템확장확인!':
                item = self.getItemName(words[1])
                if item != None and item.get('확장 이름') != '':
                    searchEnd = True
            elif func == '$아이템확장설정':
                item = self.getItemName(words[1])
                if item != None and len(words) == 4:
                    item.set('확장 이름', words[2])
                    item.setAttr('아이템속성', '팔지못함')
                    ac = item['반응이름']
                    aclist = ac
                    aclist.append(words[2])
                    acline = ''
                    for a in aclist:
                        acline += a + '\r\n'
                    item['반응이름'] = acline[:-2]
                        
            elif func == '$아이템확장설정지움':  # 낙양성:이름맨 (크래프트)
                item = self.getItemName(words[1])
                if item != None and len(words) == 3:
                    acname = item['확장 이름']
                    ac = item['반응이름']
                    aclist = ac
                    aclist.remove(acname)
                    acline = ''
                    for a in aclist:
                        acline += a + '\r\n'
                    item['반응이름'] = acline[:-2]
                    item.set('확장 이름', '')
            elif func == '$아이템삭제':
                index, cnt = getStrCnt(sline)
                if '$변수:' in line:
                    item = self.getItemName(index)
                    self.remove(item)
                else:
                    self.delItem(index, cnt)
            elif func == '$속성템주기':
                index, cnt = getStrCnt(sline)
                self.addItem(index, cnt, 1)
            elif func == '$아이템주기':
                index, cnt = getStrCnt(sline)
                self.addItem(index, cnt)
            elif func == '$아이템종류확인':
                item = self.getItemName(words[1])
                if item != None and item.getType() == sline.split()[1]:
                    continue
                else:
                    searchEnd = True
            elif func == '$무림별호조건':
                if self.getTendency(nextWords) == False:
                    searchEnd = True
            elif func == '$위치이동':
                roomName = nextWords
                if roomName == '':
                    continue
                #sline[6:].strip()
                #try:
                d = str( mob['난이도'] )
                if d != '':
                    idx = roomName.find(':')
                    if idx != -1:
                        roomName = roomName[:idx] + d + roomName[idx:]
                #except:
                #    pass
                room = getRoom(roomName)
                if room == None:
                    self.sendLine('어느곳으로도 위치이동 할 수 없습니다.')
                    continue
                self.sendLine('')
                self.enterRoom(room, '소환', '소환')
                self.lpPrompt()
            elif func == '$출력':
                self.printScript(nextWords)
            elif func == '$무공리스트확인':
                if self.checkMugongList(nextWords) == False:
                    searchEnd = True
            elif func == '$무공리스트확인!':
                if self.checkMugongList(nextWords) == True:
                    searchEnd = True
            elif func == '$무공리스트삭제':
                var = sline.split()
                for m in var:
                    if m in self.skillList:
                        self.skillList.remove(m)
            elif func == '$무공개수확인':
                if len(self.skillList) < getInt(nextWords):
                    searchEnd = True                        
            elif func == '$무공확인':
                if self.checkMugong(nextWords) == False:
                    searchEnd = True
            elif func == '$무공확인!':
                if self.checkMugong(nextWords) == True:
                    searchEnd = True
            elif func == '$무공전수':
                self.addMugong(nextWords)
            elif func == '$무공회수':
                self.delMugong(nextWords)
            elif func == '$무공시전':
                skill_found = False
                for s in self.skills:
                    #if s.name == sline[10:]:
                    if s.name == '이벤트':
                        skill_found = True
                        break
                if skill_found == False:
                    # s = MUGONG[sline[10:]].clone()
                    s = MUGONG['이벤트']
                    s.start_time = s['방어시간']
                    self.skills.append(s)
                    self._str += s._str
                    self._dex += s._dex
                    self._arm += s._arm
                    buf1, buf2, buf3 = mob.makeFightScript(s['무공스크립'], self)
                    self.sendLine(buf2)
            elif func == '$무공시전2':
                skill_found = False
                for s in self.skills:
                    #if s.name == sline[10:]:
                    if s.name == '산타의축복':
                        skill_found = True
                        break
                if skill_found == False:
                    # s = MUGONG[sline[10:]].clone()
                    s = MUGONG['산타의축복']
                    s.start_time = s['방어시간']
                    self.skills.append(s)
                    self._str += s._str
                    self._dex += s._dex
                    self._arm += s._arm
                    buf1, buf2, buf3 = mob.makeFightScript(s['무공스크립'], self)
                    self.sendLine(buf2)                                        
            elif func == '$전투강제시작':
                if mob.act == ACT_DEATH:
                    self.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
                    return
                if self.act == ACT_FIGHT:
                    if mob in self.target:
                        self.sendLine('☞ 이미 공격중이에요. ^_^')
                    else:
                        self.sendLine('☞ 현재의 비무에 신경을 집중하세요. @_@')
                else:
                    self.setFight(mob, True)
            elif func == '$전투시작':
                searchEnd = True
                if mob.act == ACT_DEATH:
                    self.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
                    return
                if mob.act == ACT_FIGHT:
                    if mob in self.target:
                        self.sendLine('☞ 이미 공격중이에요. ^_^')
                    else:
                        self.sendLine('☞ 이미 다른 사람과 전투중이라 공격할 수가 없네요')
                    continue
                if self.act == ACT_FIGHT:
                    if mob in self.target:
                        self.sendLine('☞ 이미 공격중이에요. ^_^')
                    else:
                        self.sendLine('☞ 현재의 비무에 신경을 집중하세요. @_@')
                else:
                    self.setFight(mob, True)
                    searchEnd = False
            elif func == '$몹상태확인!':
                if mob.getAct() == nextWords:
                    searchEnd = True
            elif func == '$몹상태확인':
                if mob.getAct() != nextWords:
                    searchEnd = True
            elif func == '$몹상태설정':
                mob.setAct(nextWords)
            elif func == '$체력소모':
                self.minusHP( getInt(nextWords) )
            elif func == '$체력감소':
                self.minusHP( getInt(nextWords) )
            elif func == '$내공감소':
                self.minusMP( getInt(nextWords) )
            elif func == '$변수확인':
                var = sline.split()
                c = getInt(var[1])
                if len(words) < 3 or len(var) < 3 or c > len(words) - 2:
                    searchEnd = True
                elif words[c] != var[2]:
                    searchEnd = True
            elif func == '$특성치변경':
                var = sline.split()
                c = getInt(var[2])
                cc = getInt(self.get(var[1]))
                cc = cc + c
                self.set(var[1], cc)
                self.lpPrompt()

            elif func == '$정사전환':
                if self.get('성격') == '사파':
                    self.set('성격', '정파')
                elif self.get('성격') == '정파':
                    self.set('성격', '사파')
                
            elif func == '$성별확인':
                if self.get('성별') == '남':
                    searchEnd = True
                    
            elif func == '$남자설정':
                self.set('성별', '남')
            elif func == '$여자설정':
                self.set('성별', '여')                                
            elif func == '$특성치설정':
                var = sline.split()
                c = getInt(var[2])
                self.set(var[1], c)
            elif func == '$특성치확인':
                var = sline.split()
                c = getInt(var[2])
                if self.get(var[1]) < c:
                    searchEnd = True
            elif func == '$은둔칩거설정':
                self.setEunDun()
            elif func == '$우화등선설정':
                self.setSunIn()
            elif func == '$소오강호설정':
                self.setGiIn()

            elif func == '$별호변경':
                if len(words) != 3:
                    self.sendLine('☞ 바꿀별호를 입력하세요.')
                    continue
                print(words[1])
                if self['무림별호'] == '':
                    self.sendLine('☞ 당신은 무명객입니다.')
                    continue
                if len(words[1]) < 3:
                    self.sendLine('☞ 사용하시려는 별호가 너무 짧아요.')
                    continue
                if len(words[1]) > 10:
                    self.sendLine('☞ 사용하시려는 별호가 너무 길어요.')
                    continue
                from objs.nickname import Nickname, NICKNAME
                if words[1] in NICKNAME.attr:
                    self.sendLine('☞ 다른 무림인이 사용중인 별호입니다. ^^')
                    continue
                NICKNAME.attr.__delitem__(self['무림별호'])
                NICKNAME[words[1]] = self['이름']
                NICKNAME.save()
                self['무림별호'] = words[1]
                self.do_command('귀환')
            elif func == '$기연존재확인':
                bRet, owner = ONEITEM.checkOneItemName(nextWords)
                if bRet:
                    sub['[기연소지자]'] = owner
                else:
                    searchEnd = True
            elif func == '$기연확인':
                bRet, owner = ONEITEM.checkOneItemIndex(nextWords)
                if bRet:
                    sub['[기연소지자]'] = owner
                else:
                    searchEnd = True
            elif func == '$기연확인!':
                bRet, owner = ONEITEM.checkOneItemIndex(nextWords)
                if bRet:
                    sub['[기연소지자]'] = owner
                    searchEnd = True
            elif func == '$순위기록':
                ws = sline.split(None, 2)
                if len(ws) != 3:
                    #print '!!!!'
                    searchEnd = True
                    continue
                value = self[ws[2]]
                if value == '':
                    value = -1
                rank1 = RANK.read_rank(ws[2], self['이름'])
                rank2 = RANK.write_rank(ws[2], self['이름'], value, int(ws[1]))
                #print rank1, rank2
                if rank2 == 0:
                    searchEnd = True
            elif func == '$순위갱신':
                ws = sline.split(None, 1)
                if rank1 != rank2 and rank2 == 1:
                    l = ws[1].replace('[공]', '당신')
                    self.sendLine(postPosition1(l))
                    l = ws[1].replace('[공]', self['이름'])
                    self.channel.sendToAll1(postPosition1(l), ex = self, noPrompt = True)
                    broadcast = True
            elif func == '$순위확인':
                
                ws = sline.split(None, 2)
                if len(ws) != 3:
                    searchEnd = True
                    continue
                if len(words) == 3:
                    r = getInt(words[1])
                    if words[1] == '모두':
                        msg = RANK.getRankAll(ws[2])
                        self.sendLine(msg)
                        return
                    elif r > 0:
                        if r > int(ws[1]):
                            r = int(ws[1])
                        name = RANK.getRankNum(ws[2], r)
                        if name != None:
                            sub['[순위자]'] = name
                            sub['[순위]'] = str(r)
                        else:
                            sub['[순위자]'] = '[%d위]' % r
                            searchEnd = True
                        continue
                    else:
                        name = words[1]
                else:
                    name = self['이름']
                rank1 = RANK.read_rank(ws[2], name)
                sub['[순위자]'] = name
                sub['[순위]'] = str(rank1)
                if rank1 == 0:
                    searchEnd = True
            elif func == '$순위확인!':
                ws = sline.split(None, 2)
                if len(ws) != 3:
                    searchEnd = True
                    continue
                if len(words) == 3:
                    r = getInt(words[1])
                    if words[1] == '모두':
                        msg = RANK.getRankAll(ws[2])
                        self.sendLine(msg)
                        return
                    elif r > 0:
                        if r > int(ws[1]):
                            r = int(ws[1])
                        name = RANK.getRankNum(ws[2], r)
                        if name != None:
                            sub['[순위자]'] = name
                            sub['[순위]'] = str(r)
                        else:
                            sub['[순위자]'] = '[%d위]' % r
                            searchEnd = True
                        continue
                    else:
                        name = words[1]
                else:
                    name = self['이름']
                rank1 = RANK.read_rank(ws[2], name)
                sub['[순위자]'] = name
                sub['[순위]'] = str(rank1)
                if rank1 != 0:
                    searchEnd = True
            elif func == '$레벨상위확인':
                if getInt(nextWords) > self['레벨']:
                    searchEnd = True
            elif func == '$레벨상위확인!':
                if getInt(nextWords) <= self['레벨']:
                    searchEnd = True
            elif func == '$비전설정':
                self.setAttr('비전이름', sline.split()[1])
            elif func == '$비전수련확인': # 낙양성:무공맨 (비전노인)
                if self['비전수련'] == '':
                    searchEnd = True
            elif func == '$비전종류확인':
                var = sline.split()
                if len(var) != 2:
                    print('[비전종류확인] 키워드 혹은 인자 없음')
                    return
                if var[1] not in self['비전이름']:
                    searchEnd = True
            elif func == '$비전종류확인!':
                var = sline.split()
                if len(var) != 2:
                    print('[비전종류확인] 키워드 혹은 인자 없음')
                    return
                if var[1] in self['비전이름']:
                    searchEnd = True
            elif func == '$비전수련설정확인':
                var = sline.split()
                if len(var) != 3:
                    print('[비전수련설정확인] 키워드 혹은 인자 없음')
                    return
                if var[1] != var[2]:
                    searchEnd = True
            elif func == '$비전수련가능확인!':
                var = sline.split() 
                if var[1] in var[2:]:
                    searchEnd = True
            elif func == '$비전수련삭제':
                self['비전수련'] = ''
            elif func == '$비전수련설정':
                var = sline.split()
                if len(var) != 2:
                    print('[비전수련설정] 키워드 혹은 인자 없음')
                    return
                self['비전수련'] = var[1]
            elif func == '$난이도진입기록':
                self['난이도진입시간'] = int(time.time())

            elif func == '$난이도재진입확인':
                t = self['난이도진입시간']
                if t == '':
                    t = 0
                    self['난이도진입시간'] = 0
                if int(time.time()) > t + 60 * 5:
                    searchEnd = True
                else:
                    searchEnd = False

            elif func == '$난이도재진입확인!':
                t = self['난이도진입시간']
                if t == '':
                    t = 0
                    self['난이도진입시간'] = 0
                if int(time.time()) > t + 60 * 5:
                    searchEnd = False
                else:
                    searchEnd = True

            elif func == '$스크립트호출':
                scr = nextWords
                self.INTERACTIVE = 0
                # from objs.autoscript import autoScript
                self.autoscript = self.AutoScript()
                self.autoscript.start(loadScriptFile(scr), self)
                return
            elif func == '$올숙확인':
                if getInt(self['올숙완료']) == 1:
                    searchEnd = False 
                    continue
                searchEnd = True
            elif func == '$올숙확인!':
                if getInt(self['올숙완료']) == 0:
                    searchEnd = False 
                    continue
                searchEnd = True
            elif func == '$올숙자격확인':
                if self['1 숙련도'] >= 1000 and self['2 숙련도'] >= 1000 and self['3 숙련도'] >= 1000 and self['4 숙련도'] >= 1000 and self['5 숙련도'] >= 1000: 
                    searchEnd = False 
                    continue
                searchEnd = True

            elif func == '$특성치복사저':
                oldattr = mob.attr
                mob.attr = {}
                for k in list(oldattr.keys()):
                    mob.attr[k] = oldattr[k]
                del oldattr
                mob['체력'] = self['최고체력'] 
                mob['힘'] = self['힘'] 
                mob['레벨'] = self['레벨'] + 150
                mob['맷집'] = self['맷집']
                mob['민첩성'] = self['민첩성']
                mob['회피'] = 0 
                mob['명중'] = 400                
                mob['운'] = 100
                mob['필살'] = 100
            elif func == '$특성치복사':
                oldattr = mob.attr
                mob.attr = {}
                for k in list(oldattr.keys()):
                    mob.attr[k] = oldattr[k]
                del oldattr
                mob['체력'] = self['최고체력'] * 20
                mob['힘'] = self['힘'] * 3
                mob['레벨'] = self['레벨'] + 150
                mob['맷집'] = self['맷집']
                mob['민첩성'] = self['민첩성']
                mob['회피'] = 0 
                mob['명중'] = 400                
                mob['운'] = 100
                mob['필살'] = 100
            elif func == '$특성치복사고':
                oldattr = mob.attr
                mob.attr = {}
                for k in list(oldattr.keys()):
                    mob.attr[k] = oldattr[k]
                del oldattr
                mob['체력'] = self['최고체력'] * 30
                mob['힘'] = self['힘'] * 8
                mob['레벨'] = self['레벨'] + 150
                mob['맷집'] = self['맷집'] * 3
                mob['민첩성'] = self['민첩성']
                mob['회피'] = 0 
                mob['명중'] = 400                
                mob['운'] = 100
                mob['필살'] = 100
            elif func == '$중급수련':
                oldattr = mob.attr
                mob.attr = {}
                for k in list(oldattr.keys()):
                    mob.attr[k] = oldattr[k]
                del oldattr
                mob['힘'] = self['힘']//100 + 1
                mob['레벨'] = self['레벨'] + 150
                mob['회피'] = 0
            """
            elif func == '$비무관람시작':
                pass
            elif func == '$비무관람끝':
                pass
            """
        elif sline[0] == '{':
            pass
        elif sline[0] == '}':
            pass
        else:
            if '[배울무공이름갯수]' in line:
                nCount = 0
                w = self.findObjInven(words[1])
                if w != None:
                    mlist = w['무공이름']
                    for m in mlist:
                        if m.split()[0] in self.skillList:
                            continue
                        type = m.split()[1]
                        if type != '정사':
                            if self['성격'] != type and self['성격'] != '기인' and self['성격'] != '선인':
                                continue
                        nCount += 1
                sub['[배울무공이름갯수]'] = str(nCount)
            elif '[아이템사용횟수]' in line:
                nCount = 0
                for it in self.objs:
                    if it.inUse == False and stripANSI(it['이름']) == words[1]:
                        words[1] = it['이름']
                        break
                        
                if words[1] in self.itemSkillMap:
                    nCount = self.itemSkillMap[words[1]]
                sub['[아이템사용횟수]'] = str(nCount)
            elif '[변수]' in line:
                sub['변수'] = words[1]

            for su in sub:
                line = line.replace(su, sub[su])
            self.sendLine(postPosition1(line))
    if broadcast:
        self.channel.sendToAll1('', ex = self)
    return True
