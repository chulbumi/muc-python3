# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [물품이름] [수량] 판매')
            return
        words = line.split()
        count = 1
        if len(words) >= 2:
            count = getInt(words[1])
            if count <= 0:
                count = 1
            if count > 100:
                count = 100

        mob = ob.env.findMerchant()
        if mob == None:
            ob.sendLine('☞ 그런 물건을 살 상인이 없어요. ^_^')
            return
        if mob['물건구입'] == '':
            ob.sendLine('☞ 그런 물건을 살 상인이 없어요. ^_^')
            return

        w = mob['물건구입'].split()
        percent = int(w[1])

        if line == '속성0':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
                if item['옵션'] != '':
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if line == '속성1':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
                if item['옵션'] != None and len(item['옵션']) > 2:
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if line == '속성2':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
                if item['옵션'] != None and len(item['옵션']) > 3:
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if line == '속성3':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
                if item['옵션'] != None and len(item['옵션']) > 4:
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return

        if line == '일반':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
                if item['옵션'] == None:
                    pass
                elif item['옵션'] != None and len(item['옵션']) != 0:
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return

        if line == '장비':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
                if item['종류'] != '방어구' and item['종류'] != '무기':
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return

        if line == '모두':
            c = 0
            objs = copy.copy(ob.objs)
            for item in objs:
                if item.checkAttr('아이템속성', '출력안함'):
                    continue
                if item.inUse == True:
                    continue
                if item.checkAttr('아이템속성', '팔지못함'):
                    continue
 
                p = (getInt(item['판매가격']) * percent) // 100
                op = item.getOption()
                if op != None:
                    p = int( p * (len(op) * 1.2) )
                ob['은전'] += p
                ob.remove(item)
                if item.isOneItem():
                    ONEITEM.destroy(item.index)
                ob.sendLine('당신이 %s 1개를 은전 %d개에 판매합니다.' % ( item.getNameA(), p))
                #ob.sendRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), p))
                del item
                c += 1
            if c == 0:
                ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return

        name, order = getNameOrder(words[0])
        if order != 1:
            count = 1

        item = ob.findObjInven(name, order)
        if item == None:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if item.checkAttr('아이템속성', '출력안함'):
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if item.inUse == True:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if item.checkAttr('아이템속성', '팔지못함'):
            ob.sendLine('☞ 그 아이템은 팔 수가 없어요~')
            return
        
        c = 0
        sum = 0
        p = (getInt(item['판매가격']) * percent) // 100
        obj = item
        for i in range(count):
            p = (getInt(obj['판매가격']) * percent) // 100
            op = obj.getOption()
            if op != None:
                p = int( p * (len(op) * 1.3) )
            ob['은전'] += p
            sum += p
            c += 1
            ob.remove(obj)
            if obj.isOneItem():
                ONEITEM.destroy(obj.index)
            del obj
            if order != 1:
                break
            obj = ob.findObjInven(name)
            if obj == None:
                break
            if obj.inUse == True:
                break
        if c == 0:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
        else:
            ob.sendLine('당신이 %s %d개를 은전 %d개에 판매합니다.' % ( item.getNameA(), c, sum))
            #ob.sendRoom('%s %s %d개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), c, sum))
            ob.sendFightScriptRoom('%s %s 1개를 은전 %d개에 판매합니다.' % ( ob.han_iga(), item.getNameA(), sum))

