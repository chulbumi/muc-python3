# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [물품이름] [수량] 구입')
            return
        words = line.split()
        count = 1
        if len(words) >= 2:
            count = getInt(words[1])
            if count <= 0:
                count = 1
            if count > 50:
                count = 50
        found = False
        mob = ob.env.findMerchant()
        if mob == None:
            ob.sendLine('☞ 품목을 보여줄 상인이 없어요. ^^')
            return
        if words[0] == '체력약' and '체력약' in ob.alias:
            words[0] = ob.alias['체력약']
            if '체력약개수' in ob.alias:
                count = getInt(ob.alias['체력약개수'])
                for itm in ob.objs:
                    if itm['이름'] == words[0]:
                        count -= 1
            if count <= 0:
                ob.sendLine('☞ 구매할 물품이 충분합니다. ^_^')
                return
            if count > 200:
                count = 200 
        if words[0] == '내공약' and '내공약' in ob.alias:
            words[0] = ob.alias['내공약']
            if '내공약개수' in ob.alias:
                count = getInt(ob.alias['내공약개수'])
                for itm in ob.objs:
                    if itm['이름'] == words[0]:
                        count -= 1
            if count <= 0:
                ob.sendLine('☞ 구매할 물품이 충분합니다. ^_^')
                return
            if count > 200:
                count = 200 
        item = None
        for l in mob['물건판매']:
            w = l.split()
            index = w[0]
            percent = int(w[1])
            item = getItem(index)
            if item == None:
                continue
            if item['이름'] == words[0] or words[0] in item['반응이름']:
                found = True
                break
        if found == False:
            ob.sendLine('☞ 그런 물건은 팔지 않아요. ^_^')
            return
        if item['종류'] == '호위':
            self.buyGuardMob(ob, item)
            return
        c = 0
        p = getInt(item['판매가격']) * 100 // percent
        for i in range(count):
            if ob.getItemCount() >= getInt(MAIN_CONFIG['사용자아이템갯수']):
                if c == 0:
                    ob.sendLine('☞ 자네가 가질 물품의 한계라네')
                    return
                break
            if ob.getItemWeight() + item['무게'] > ob.getStr() * 10:
                if c == 0:
                    ob.sendLine('☞ 무거워서 더 이상 가질 수 없어요. ^^')
                    return
                break
            money = ob['은전']
            if money < p:
                if c == 0:
                    ob.sendLine('☞ 돈이 모자라네요. ^^')
                    return
                break
            money -= p
            ob['은전'] = money
            c += 1
            obj = copy.deepcopy(item)
            ob.insert(obj)
        if c == 0:
            ob.sendLine('☞ 무거워서 더 이상 가질 수 없어요. ^^')
        else:
            ob.sendLine('당신이 %s %d개를 은전 %d개에 구입합니다.' % ( item.getNameA(), c, c * p))
            #ob.sendRoom('%s %s %d개를 은전 %d개에 구입합니다.' % (ob.han_iga(), item.getNameA(), c, c * p))
            ob.sendFightScriptRoom('%s %s %d개를 은전 %d개에 구입합니다.' % (ob.han_iga(), item.getNameA(), c, c * p))
            
    def buyGuardMob(self, ob, item):
        chI = ob['성격']
        chU = item['구매속성']
        if chI != '기인' and chI != '선인':
            if chU == '사파' and chI != chU:
                ob.sendLine('☞ 해당 호위는 사파원만 사용 가능합니다.')
                return
            if chU == '정파' and chI != chU:
                ob.sendLine('☞ 해당 호위는 정파원만 사용 가능합니다.')
                return
        lines = item['구매조건']
        if len(lines) == 0:
            ob.sendLine('☞ 해당 호위를 살수가 없습니다.')
            return
            
        #구매레벨
        maxLv = 0
        count = 0
        for obj in ob.objs:
            if obj['종류'] == '호위':
                if obj['이름'] == item['이름']:
                    count += 1
                lv = obj['구매레벨']
                if lv > maxLv:
                    maxLv = lv
        if item['구매레벨'] < maxLv:
            ob.sendLine('☞ 해당 호위를 살수가 없습니다.(레벨)')
            return
        limit = 0
        for attr in item['아이템속성']:
            if attr.find('소지한계') == 0:
                limit = getInt(attr.split()[1])
                break
        
        if count >= limit:
            ob.sendLine('☞ 해당 호위를 살수가 없습니다.(동종개수제한)')
            return
            
        match = False
        for line in lines:
            words = line.split()
            l = len(words)
            if l == 2:
                if words[0] == '약초':
                    n = self.getHurbNum(ob)
                else:
                    n = self.getGuardNum(ob, words[0])
                if n < int(words[1]):
                    continue
                gName = words[0]
                nNum = int(words[1])
                match = True
                break
            elif l == 3:
                n = self.getGuardNum(ob, words[0])
                if n < 1:
                    continue
                n = self.getHurbNum(ob)
                if n < int(words[2]):
                    continue
                match = True
                gName = words[1]
                nNum = int(words[2])
                break
            else:
                continue
        if match == False:
            ob.sendLine('☞ 해당 호위를 살수가 없습니다.')
            return
        if gName == '약초':
            self.delHerb(ob, nNum)
        else:
            n = 0
            objs = copy.copy(ob.objs)
            for obj in objs:
                if obj['이름'] == gName:
                    ob.remove(obj)
                    n += 1
                    if n == nNum:
                        break
                        
        g = item.clone()
        g.hp = g['체력']
        ob.insert(g)
        
        ob.sendLine('당신이 %s 구입합니다.' % item.han_obj())
        ob.sendFightScriptRoom('%s %s 구입합니다.' % (ob.han_iga(), item.han_obj()))
        #ob.sendRoom('%s %s 구입합니다.' % (ob.han_iga(), item.han_obj()))
        #ob.sendLine('☞ 아직 호위를 살 수 없어요^^;;')
        
    def getGuardNum(self, ob, name):
        n = 0
        for obj in ob.objs:
            if obj['이름'] == name:
                n += 1
        return n
        
    def delHerb(self, ob, c):
        n = 0
        herbs = ['합성1', '합성2', '합성3', '합성4', '합성5', '합성6', '합성7', '합성8', '합성9']
        objs = copy.copy(ob.objs)
        for obj in objs:
            if obj.index in herbs:
                n += 1
                ob.remove(obj)
                if c == n:
                    break
        
    def getHurbNum(self, ob):
        n = 0
        herbs = ['합성1', '합성2', '합성3', '합성4', '합성5', '합성6', '합성7', '합성8', '합성9']
        for obj in ob.objs:
            if obj.index in herbs:
                n += 1
        return n
