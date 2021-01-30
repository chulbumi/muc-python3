# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [아이템 이름] 해제')
            return
        msg = ''
        if line == '모두' or line == '전부':
            cnt = 0
            i = 0
            for obj in ob.objs:
                if obj.inUse:
                    obj.inUse = False
                    ob.armor -= getInt(obj['방어력'])
                    ob.attpower -= getInt(obj['공격력'])
                    option = obj.getOption()
                    if option != None:
                        for op in option:
                            if op == '힘':
                                ob._str -= option[op]
                            elif op == '민첩성':
                                ob._dex -= option[op]
                            elif op == '맷집':
                                ob._arm -= option[op]
                            elif op == '체력':
                                ob._maxhp -= option[op]
                            elif op == '내공':
                                ob._maxmp -= option[op]
                            elif op == '필살':
                                ob._critical -= option[op]
                            elif op == '운':
                                 ob._criticalChance -= option[op]
                            elif op == '회피':
                                ob._miss -= option[op]
                            elif op == '명중':
                                ob._hit -= option[op]
                            elif op == '경험치':
                                ob._exp -= option[op]
                            elif op == '마법발견':
                                ob._magicChance -= option[op]
                    if obj['종류'] == '무기':
                        ob.weaponItem = None
                    ob.sendLine('당신이 [36m' + obj.get('이름') + '[37m' + han_obj(obj.getStrip('이름')) + ' 착용해제 합니다.')
                    #ob.sendRoom('%s %s 착용해제 합니다.' % (ob.han_iga(), obj.han_obj()))
                    msg += '%s %s 착용해제 합니다.\r\n' % (ob.han_iga(), obj.han_obj())
                    cnt = cnt + 1
                   
            if cnt == 0:
                ob.sendLine('☞ 착용중인 장비가 없어요.')
                return
            else:
                ob.sendRoom(msg[:-2])
        else:
            item = ob.findObjInUse(line)

            if item == None:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
                return
            if item.inUse == False:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
                return

            item.inUse = False
            ob.armor -= getInt(item['방어력'])
            ob.attpower -= getInt(item['공격력'])
            option = item.getOption()
            if option != None:
                for op in option:
                    if op == '힘':
                        ob._str -= option[op]
                    elif op == '민첩성':
                        ob._dex -= option[op]
                    elif op == '맷집':
                        ob._arm -= option[op]
                    elif op == '체력':
                        ob._maxhp -= option[op]
                    elif op == '내공':
                        ob._maxmp -= option[op]
                    elif op == '필살':
                        ob._critical -= option[op]
                    elif op == '운':
                         ob._criticalChance -= option[op]
                    elif op == '회피':
                        ob._miss -= option[op]
                    elif op == '명중':
                        ob._hit -= option[op]
                    elif op == '경험치':
                        ob._exp -= option[op]
                    elif op == '마법발견':
                        ob._magicChance -= option[op]
            if item['종류'] == '무기':
                    ob.weaponItem = None
            ob.sendLine('당신이 [36m' + item.get('이름') + '[37m' + han_obj(item.getStrip('이름')) + ' 착용해제 합니다.')
            ob.sendRoom('%s %s 착용해제 합니다.' % (ob.han_iga(), item.han_obj()))

