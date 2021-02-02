# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        savedSet = ob['세트기억']
        if savedSet == '':
            ob.sendLine('☞ 사용법: [세트기억] 후 [세트착용]')
            return
        
        msg = ''

        cnt = 0
        i = 0
        for obj in ob.objs:
            #ob.objs.remove(ob.objs[i])
            #obj.move_object(ob.env)
            if obj.inUse:
                continue
            if obj.get('종류') != '방어구' and obj.get('종류') != '무기':
                continue
            if ob.checkArmed(obj.get('계층')):
                continue
            if obj.checkAttr('아이템속성', '올숙천무기'):
                if self.checkSuk(ob, 1000) == False:
                    continue
            if obj.checkAttr('아이템속성', '올숙이천무기'):
                if self.checkSuk(ob, 2000) == False:
                    continue
            if savedSet not in obj['반응이름']:
                continue
            ob.armor += getInt(obj['방어력'])
            ob.attpower += getInt(obj['공격력'])
            option = obj.getOption()
            if option != None:
                for op in option:
                    if op == '힘':
                        ob._str += option[op]
                    elif op == '민첩성':
                        ob._dex += option[op]
                    elif op == '맷집':
                        ob._arm += option[op]
                    elif op == '체력':
                        ob._maxhp += option[op]
                    elif op == '내공':
                        ob._maxmp += option[op]
                    elif op == '필살':
                        ob._critical += option[op]
                    elif op == '운':
                        ob._criticalChance += option[op]
                    elif op == '회피':
                        ob._miss += option[op]
                    elif op == '명중':
                        ob._hit += option[op]
                    elif op == '경험치':
                        ob._exp += option[op]
                    elif op == '마법발견':
                        ob._magicChance += option[op]

            if obj['종류'] == '무기':
                ob.weaponItem = obj
            s = obj.getUseScript()
            if s == '':
                ob.sendLine('당신이 [36m' + obj.han_obj() + '[37m 착용합니다.')
                #ob.sendRoom('%s %s 착용합니다.' % (ob.han_iga(), obj.han_obj()))
                msg += '%s %s 착용합니다.\r\n' % (ob.han_iga(), obj.han_obj())
            else:
                ob.sendLine('당신이 ' + s)
                #ob.sendRoom('%s %s' % (ob.han_iga(),s))
                msg += '%s %s\r\n' % (ob.han_iga(),s)
                
            obj.inUse = True
            cnt = cnt + 1
               
        if cnt == 0:
            ob.sendLine('☞ 더이상 착용할 장비가 없어요.')
        else:
            ob.sendRoom(msg[:-2])
        

    def checkSuk(self, ob, min):
        if ob['1 숙련도'] >= min and ob['2 숙련도'] >= min and ob['3 숙련도'] >= min and ob['4 숙련도'] >= min and ob['5 숙련도'] >= min:
            return True
        return False
